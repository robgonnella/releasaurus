use color_eyre::eyre::Result;
use log::*;
use regex::Regex;
use std::{collections::HashMap, env};
use tempfile::TempDir;

use crate::{
    cli, config,
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, PENDING_LABEL},
        traits::Forge,
        types::{
            CreatePrRequest, GetPrRequest, PrLabelsRequest, UpdatePrRequest,
        },
    },
    processor::{
        cliff::CliffProcessor, config::ChangelogConfig, types::Output,
    },
    repo::Repository,
};

fn create_or_update_pr(
    title: &str,
    body: &str,
    release_branch: &str,
    repo: &Repository,
    forge: Box<dyn Forge>,
) -> Result<()> {
    let head_branch =
        format!("{}{}", DEFAULT_PR_BRANCH_PREFIX, repo.default_branch);
    let base_branch = repo.default_branch.clone();

    repo.add_all()?;
    repo.commit(title)?;
    repo.push_branch(release_branch)?;

    let req = GetPrRequest {
        head_branch: head_branch.clone(),
        base_branch: base_branch.clone(),
    };

    info!("looking for existing release pull request");
    let result = forge.get_open_release_pr(req)?;

    let pr = match result {
        Some(pr) => {
            info!("updating pr {}", pr.number);
            let req = UpdatePrRequest {
                pr_number: pr.number,
                title: title.to_string(),
                body: body.to_string(),
            };

            forge.update_pr(req)?;
            pr
        }
        None => {
            info!("creating pull request: {title}");
            let req = CreatePrRequest {
                title: title.to_string(),
                body: body.to_string(),
                head_branch,
                base_branch,
            };

            forge.create_pr(req)?
        }
    };

    info!("setting labels for pr {}", pr.number);

    let req = PrLabelsRequest {
        pr_number: pr.number,
        labels: vec![PENDING_LABEL.into()],
    };

    forge.replace_pr_labels(req)
}

struct ManifestProcessingResult {
    pub title: String,
    pub body: Vec<String>,
    pub releasable: bool,
}

fn process_manifest(
    manifest: HashMap<String, Output>,
    repo: &Repository,
) -> ManifestProcessingResult {
    let mut title = format!("chore({}): release", repo.default_branch);
    let mut body = vec![];
    let mut releasable = false;
    let version_line_re = Regex::new(r"(?m)^#\s.+\w.+$").unwrap();

    for (name, info) in manifest {
        info!(
            "\n\n{}\n  current_version: {:?}\n  next_version: {:?}\n  projected_release_version: {:?}",
            name, info.current_version, info.next_version, info.next_version,
        );

        if let Some(version) = info.next_version {
            info!("{name} is releasable: next version: {version}");
            releasable = true;

            // replace first line as it will be part of drop down title
            let changelog = version_line_re.replace(&info.changelog, "");

            let drop_down = format!(
                "<details><summary>{}</summary><br>{}</details>",
                version, changelog
            );
            body.push(drop_down);
            title = format!("{} {}", title, version)
        }
    }

    ManifestProcessingResult {
        title,
        body,
        releasable,
    }
}

pub fn execute(args: &cli::Args) -> Result<()> {
    let remote = args.get_remote()?;
    let forge = remote.get_forge()?;
    let remote_config = forge.config();
    let tmp_dir = TempDir::new()?;

    info!(
        "cloning repository {} to {}",
        remote_config.repo,
        tmp_dir.path().display()
    );

    let repo = Repository::new(tmp_dir.path(), remote_config.clone())?;

    info!(
        "switching directory to cloned repository: {}",
        tmp_dir.path().display(),
    );

    env::set_current_dir(tmp_dir.path())?;

    info!("loading configuration");

    let cli_config = config::load_config()?;

    let release_branch =
        format!("{}{}", DEFAULT_PR_BRANCH_PREFIX, repo.default_branch);

    debug!("setting up release branch: {release_branch}");
    repo.create_branch(&release_branch)?;
    repo.switch_branch(&release_branch)?;

    let mut manifest: HashMap<String, Output> = HashMap::new();

    for package in cli_config.packages {
        let tag_prefix = package.tag_prefix.clone().unwrap_or("v".into());
        info!(
            "processing changelog for package path: {}, tag_prefix: {}",
            package.path, tag_prefix
        );
        let starting_sha =
            repo.get_latest_tagged_starting_point(&tag_prefix)?;

        let changelog_config = ChangelogConfig {
            package_path: package.path.clone(),
            header: cli_config.changelog.header.clone(),
            body: cli_config.changelog.body.clone(),
            footer: cli_config.changelog.footer.clone(),
            commit_link_base_url: remote_config.commit_link_base_url.clone(),
            release_link_base_url: remote_config.release_link_base_url.clone(),
            tag_prefix: package.tag_prefix,
            since_commit: starting_sha,
        };

        let processor = CliffProcessor::new(changelog_config)?;
        let output = processor.write_changelog()?;
        manifest.insert(package.path, output);
    }

    let manifest_result = process_manifest(manifest, &repo);

    if !manifest_result.releasable {
        info!("no releasable commits found");
        return Ok(());
    }

    if args.dry_run {
        info!("dry-run: skipping remote update");
        return Ok(());
    }

    create_or_update_pr(
        &manifest_result.title,
        &manifest_result.body.join("\n"),
        &release_branch,
        &repo,
        forge,
    )?;

    Ok(())
}
