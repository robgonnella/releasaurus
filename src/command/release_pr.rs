//! Release pull request creation command implementation.

use color_eyre::eyre::eyre;
use log::*;
use std::{collections::HashMap, path::Path};

use crate::{
    analyzer::Analyzer,
    command::common,
    config::{Config, ReleaseType},
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, PENDING_LABEL},
        request::{
            CreateBranchRequest, CreatePrRequest, FileChange, FileUpdateType,
            GetPrRequest, PrLabelsRequest, UpdatePrRequest,
        },
        traits::Forge,
    },
    result::{PendingReleaseError, ReleasablePackage, Result},
    updater::framework::Framework,
};

#[derive(Debug, Clone)]
struct ReleasePr {
    pub title: String,
    pub body: String,
    pub file_changes: Vec<FileChange>,
}

/// Analyze commits since last tags, generate changelogs, update version files,
/// and create or update release PR.
pub async fn execute(
    forge: Box<dyn Forge>,
    prerelease_override: Option<String>,
) -> Result<()> {
    let mut config = forge.load_config().await?;
    let repo_name = forge.repo_name();
    let config = common::process_config(&repo_name, &mut config);

    let releasable_packages = get_releasable_packages(
        &config,
        forge.as_ref(),
        prerelease_override.clone(),
    )
    .await?;

    debug!("releasable packages: {:#?}", releasable_packages);

    let prs_by_branch = gather_release_prs_by_branch(
        &releasable_packages,
        forge.as_ref(),
        &config,
    )
    .await?;

    if prs_by_branch.is_empty() {
        return Ok(());
    }

    create_branch_release_prs(prs_by_branch, forge.as_ref()).await?;

    Ok(())
}

async fn create_branch_release_prs(
    prs_by_branch: HashMap<String, Vec<ReleasePr>>,
    forge: &dyn Forge,
) -> Result<()> {
    let default_branch = forge.default_branch().await?;
    // create a single pr per branch
    for (release_branch, prs) in prs_by_branch {
        let single_pr = prs.len() == 1;

        let mut title = format!("chore({default_branch}): release");
        let mut body: Vec<String> = vec![];
        let mut file_changes: Vec<FileChange> = vec![];

        for pr in prs {
            if single_pr {
                title = pr.title;
            }

            body.push(pr.body);
            file_changes.extend(pr.file_changes);
        }

        let in_process_release_req = GetPrRequest {
            base_branch: default_branch.clone(),
            head_branch: release_branch.clone(),
        };

        let pending_release =
            forge.get_merged_release_pr(in_process_release_req).await?;

        if let Some(pr) = pending_release {
            error!("pending release: {:#?}", pr);
            return Err(PendingReleaseError {
                branch: release_branch.clone(),
                pr_number: pr.number,
            }
            .into());
        }

        info!("creating / updating release branch: {release_branch}");
        forge
            .create_release_branch(CreateBranchRequest {
                branch: release_branch.clone(),
                message: title.clone(),
                file_changes,
            })
            .await?;

        info!("searching for existing pr for branch {release_branch}");
        let pr = forge
            .get_open_release_pr(GetPrRequest {
                head_branch: release_branch.clone(),
                base_branch: default_branch.clone(),
            })
            .await?;

        let pr = if let Some(pr) = pr {
            forge
                .update_pr(UpdatePrRequest {
                    pr_number: pr.number,
                    title,
                    body: body.join("\n"),
                })
                .await?;
            info!("updated existing release-pr: {}", pr.number);
            pr
        } else {
            let pr = forge
                .create_pr(CreatePrRequest {
                    head_branch: release_branch,
                    base_branch: default_branch.clone(),
                    title,
                    body: body.join("\n"),
                })
                .await?;
            info!("created release-pr: {}", pr.number);
            pr
        };
        info!("setting pr labels: {PENDING_LABEL}");

        forge
            .replace_pr_labels(PrLabelsRequest {
                pr_number: pr.number,
                labels: vec![PENDING_LABEL.into()],
            })
            .await?;
    }

    Ok(())
}

async fn gather_release_prs_by_branch(
    releasable_packages: &[ReleasablePackage],
    forge: &dyn Forge,
    config: &Config,
) -> Result<HashMap<String, Vec<ReleasePr>>> {
    let default_branch = forge.default_branch().await?;

    let mut prs_by_branch: HashMap<String, Vec<ReleasePr>> = HashMap::new();

    for pkg in releasable_packages.iter() {
        let mut file_changes =
            Framework::update_package(forge, pkg, releasable_packages).await?;

        let mut title =
            format!("chore({}): release {}", default_branch, pkg.name);

        let mut release_branch =
            format!("{DEFAULT_PR_BRANCH_PREFIX}-{}", default_branch);

        if config.separate_pull_requests {
            release_branch = format!("{release_branch}-{}", pkg.name);
        }

        let mut start_details = "<details>";

        // auto-open dropdown if there's only one package
        // or if separate_pull_requests
        if releasable_packages.len() == 1 || config.separate_pull_requests {
            start_details = "<details open>";
        }

        if pkg.release.tag.is_none() {
            return Err(eyre!(
                "Projected release should have a projected tag but failed to detect one. Please report this issue here: https://github.com/robgonnella/releasaurus/issues"
            ));
        }

        let tag = pkg.release.tag.clone().unwrap();

        title = format!("{title} {}", tag.name);

        // create the drop down
        let body = format!(
            "{start_details}<summary>{}</summary>\n\n{}</details>",
            tag.name, pkg.release.notes
        );

        let changelog_path = Path::new(&pkg.workspace_root)
            .join(&pkg.path)
            .join("CHANGELOG.md")
            .display()
            .to_string()
            .replace("./", "");

        file_changes.push(FileChange {
            content: format!("{}\n\n", pkg.release.notes),
            path: changelog_path,
            update_type: FileUpdateType::Prepend,
        });

        let prs = prs_by_branch.get_mut(&release_branch);

        if let Some(prs) = prs {
            prs.push(ReleasePr {
                title,
                body,
                file_changes: file_changes.clone(),
            })
        } else {
            prs_by_branch.insert(
                release_branch.clone(),
                vec![ReleasePr {
                    title,
                    body,
                    file_changes: file_changes.clone(),
                }],
            );
        };
    }

    Ok(prs_by_branch)
}

async fn get_releasable_packages(
    config: &Config,
    forge: &dyn Forge,
    prerelease_override: Option<String>,
) -> Result<Vec<ReleasablePackage>> {
    let repo_name = forge.repo_name();
    let remote_config = forge.remote_config();

    let mut manifest: Vec<ReleasablePackage> = vec![];

    for package in config.packages.iter() {
        let tag_prefix = common::get_tag_prefix(package, &repo_name);

        info!(
            "processing package: \n\tname: {}, \n\tworkspace_root: {}, \n\tpath: {}, \n\ttag_prefix: {}",
            package.name, package.workspace_root, package.path, tag_prefix
        );

        let current_tag = forge.get_latest_tag_for_prefix(&tag_prefix).await?;

        info!(
            "package_name: {}, current tag {:#?}",
            package.name, current_tag
        );

        let current_sha = current_tag.clone().map(|t| t.sha);

        let package_full_path = Path::new(&package.workspace_root)
            .join(&package.path)
            .display()
            .to_string()
            .replace("./", "");

        let commits =
            forge.get_commits(&package_full_path, current_sha).await?;

        info!("processing commits for package: {}", package.name);

        let analyzer_config = common::generate_analyzer_config(
            config,
            &remote_config,
            package,
            tag_prefix.clone(),
            prerelease_override.clone(),
        );

        let analyzer = Analyzer::new(analyzer_config)?;
        let release = analyzer.analyze(commits, current_tag)?;

        info!("package: {}, release: {:#?}", package.name, release);

        if release.is_none() {
            info!("nothing to release for package: {}", package.name);
            continue;
        }

        let release = release.unwrap();

        let release_type =
            package.release_type.clone().unwrap_or(ReleaseType::Generic);

        manifest.push(ReleasablePackage {
            name: package.name.clone(),
            path: package.path.clone(),
            workspace_root: package.workspace_root.clone(),
            release_type,
            release,
        });
    }

    Ok(manifest)
}
