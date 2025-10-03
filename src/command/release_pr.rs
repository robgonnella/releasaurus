//! Release pull request creation command implementation.

use log::*;
use std::{collections::HashMap, path::Path};

use crate::{
    analyzer::{Analyzer, config::AnalyzerConfig, release::Release},
    command::common,
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, PENDING_LABEL, Remote},
        request::{
            CreateBranchRequest, CreatePrRequest, FileChange, FileUpdateType,
            GetPrRequest, PrLabelsRequest, UpdatePrRequest,
        },
    },
    result::Result,
    updater::manager::UpdaterManager,
};

/// Execute release-pr command to analyze commits and create release pull request.
pub async fn execute(remote: Remote) -> Result<()> {
    let remote_config = remote.get_config();
    let forge = remote.get_forge().await?;
    let file_loader = remote.get_file_loader().await?;
    let config = forge.load_config().await?;
    let mut manifest: HashMap<String, Release> = HashMap::new();

    for package in config.packages.iter() {
        let tag_prefix = common::get_tag_prefix(package);
        info!(
            "processing package: path: {}, tag_prefix: {}",
            package.path, tag_prefix
        );
        let current_tag = forge.get_latest_tag_for_prefix(&tag_prefix).await?;

        info!("path: {}, current tag {:#?}", package.path, current_tag);

        let current_sha = current_tag.clone().map(|t| t.sha);
        let commits = forge.get_commits(&package.path, current_sha).await?;

        info!("processing commits for package: {}", package.path);

        let analyzer_config = AnalyzerConfig {
            body: config.changelog.body.clone(),
            release_link_base_url: remote_config.release_link_base_url.clone(),
            tag_prefix: Some(tag_prefix),
        };

        let analyzer = Analyzer::new(analyzer_config)?;
        let release = analyzer.analyze(commits, current_tag)?;

        info!("package path: {}, release: {:#?}", package.path, release);

        if let Some(release) = release {
            manifest.insert(package.path.clone(), release);
        }
    }

    debug!("manifest: {:#?}", manifest);

    let default_branch = forge.default_branch().await?;
    let release_branch =
        format!("{DEFAULT_PR_BRANCH_PREFIX}-{}", default_branch);

    let include_title_version = config.packages.len() == 1;
    let mut title = format!("chore({}): release", default_branch);

    let mut file_updates: Vec<FileChange> = vec![];
    let mut body = vec![];
    let mut start_tag = "<details>";

    // auto-open dropdown if there's only one package
    if manifest.len() == 1 {
        start_tag = "<details open>";
    }

    for (path, release) in manifest.iter() {
        if let Some(tag) = release.tag.clone() {
            if include_title_version {
                title = format!("{title} {}", tag.name);
            }

            // create the drop down
            let drop_down = format!(
                "{start_tag}<summary>{}</summary>\n\n{}</details>",
                tag.name, release.notes
            );

            body.push(drop_down);

            file_updates.push(FileChange {
                content: format!("{}\n\n", release.notes),
                path: Path::new(path)
                    .join("CHANGELOG.md")
                    .display()
                    .to_string(),
                update_type: FileUpdateType::Prepend,
            });
        }
    }

    let repo_name = forge.repo_name();

    let mut update_manager = UpdaterManager::new(&repo_name);

    if let Some(changes) = update_manager
        .update_packages(&manifest, &config, file_loader.as_ref())
        .await?
    {
        file_updates.extend(changes);
    }

    if !manifest.is_empty() {
        info!("creating / updating release branch: {release_branch}");
        forge
            .create_release_branch(CreateBranchRequest {
                branch: release_branch.clone(),
                message: title.clone(),
                file_changes: file_updates,
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
                    base_branch: default_branch,
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
