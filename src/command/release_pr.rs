use color_eyre::eyre::{ContextCompat, Result};
use log::*;
use std::collections::HashMap;

use crate::{
    analyzer::{cliff::CliffAnalyzer, types::Output},
    cli,
    command::common,
    config,
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, PENDING_LABEL},
        traits::Forge,
        types::{CreatePrRequest, GetPrRequest, UpdatePrRequest},
    },
    repo::Repository,
    updater::manager::UpdaterManager,
};

struct PrContent {
    pub title: String,
    pub body: Vec<String>,
    pub releasable: bool,
}

pub fn execute(args: &cli::Args) -> Result<()> {
    let remote = args.get_remote()?;
    let forge = remote.get_forge()?;

    let (repo, tmp_dir) = common::setup_repository(forge.as_ref())?;
    let cli_config = common::load_configuration(tmp_dir.path())?;

    let release_branch =
        common::setup_release_branch(&repo, DEFAULT_PR_BRANCH_PREFIX)?;

    let manifest = process_packages(&repo, &cli_config, forge.config())?;
    let pr_content = create_pr_content(manifest, &repo)?;

    if !pr_content.releasable {
        info!("no releasable commits found");
        return Ok(());
    }

    if args.dry_run {
        info!("dry-run: skipping remote update");
        return Ok(());
    }

    create_or_update_pr(
        &pr_content.title,
        &pr_content.body.join("\n"),
        &release_branch,
        &repo,
        forge,
    )?;

    // Keep tmp_dir alive until the end to prevent cleanup
    drop(tmp_dir);

    Ok(())
}

fn process_packages(
    repo: &Repository,
    cli_config: &config::CliConfig,
    remote_config: &crate::forge::config::RemoteConfig,
) -> Result<HashMap<String, Output>> {
    let mut manifest: HashMap<String, Output> = HashMap::new();

    for package in &cli_config.packages {
        let output =
            process_single_package(package, repo, cli_config, remote_config)?;
        manifest.insert(package.path.clone(), output);
    }

    // Update package manifest files (Cargo.toml, package.json, setup.py, etc.)
    // with the new versions using the language-agnostic updater system.
    // This step ensures that package files are updated consistently across
    // different programming languages and package managers.
    info!(
        "Updating package manifest files with new versions across all detected frameworks"
    );
    let mut updater_manager = UpdaterManager::new(repo.workdir()?);

    match updater_manager.update_packages(&manifest, cli_config) {
        Ok(stats) => {
            info!(
                "✓ Package update completed successfully: {} of {} packages updated",
                stats.updated_packages, stats.total_packages
            );
            debug!("{stats}");
        }
        Err(e) => {
            warn!("Failed to update package manifest files: {}", e);
            warn!(
                "Continuing with PR creation - changelog generation was successful"
            );
            // Continue with PR creation even if file updates fail since the core
            // functionality (changelog generation and version detection) worked.
            // The PR will still be created with the correct changelog content.
        }
    }

    Ok(manifest)
}

fn process_single_package(
    package: &config::CliPackageConfig,
    repo: &Repository,
    cli_config: &config::CliConfig,
    remote_config: &crate::forge::config::RemoteConfig,
) -> Result<Output> {
    let tag_prefix = common::get_tag_prefix(package);

    common::log_package_processing(&package.path, &tag_prefix);

    let starting_point = repo.get_latest_tagged_starting_point(&tag_prefix)?;

    let changelog_config = common::create_changelog_config(
        package,
        cli_config,
        remote_config,
        starting_point,
        String::from(repo.workdir_as_str()),
    );

    let analyzer = CliffAnalyzer::new(changelog_config)?;
    analyzer.write_changelog()
}

fn create_pr_content(
    manifest: HashMap<String, Output>,
    repo: &Repository,
) -> Result<PrContent> {
    let mut title = format!("chore({}): release", repo.default_branch);
    let mut include_title_version = false;
    let mut body = vec![];
    let mut releasable = false;

    let mut start_tag = "<details>";

    // auto-open dropdown if there's only one package
    if manifest.len() == 1 {
        include_title_version = true;
        start_tag = "<details open>";
    }

    for (name, info) in manifest {
        debug!(
            "\n\n{}\n  current_version: {:?}\n  next_version: {:?}\n  projected_release_version: {:?}",
            name, info.current_version, info.next_version, info.next_version,
        );

        // only keep packages that have releasable commits
        if let Some(version) = info.next_version {
            info!("{name} is releasable: next version: {}", version.semver);
            releasable = true;

            let notes = info
                .projected_release
                .wrap_err("failed to find projected release")?
                .notes;

            // create the drop down
            let drop_down = format!(
                "{start_tag}<summary>{}</summary>{}</details>",
                version.semver, notes
            );

            body.push(drop_down);

            if include_title_version {
                title = format!("{} {}", title, version.semver)
            }
        }
    }

    Ok(PrContent {
        title,
        body,
        releasable,
    })
}

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

    common::commit_and_push_changes(repo, title, release_branch)?;

    let pr = find_or_create_pr(
        title,
        body,
        &head_branch,
        &base_branch,
        forge.as_ref(),
    )?;

    common::update_pr_labels(
        forge.as_ref(),
        pr.number,
        vec![PENDING_LABEL.into()],
    )?;

    Ok(())
}

fn find_or_create_pr(
    title: &str,
    body: &str,
    head_branch: &str,
    base_branch: &str,
    forge: &dyn Forge,
) -> Result<crate::forge::types::ReleasePullRequest> {
    let req = GetPrRequest {
        head_branch: head_branch.to_string(),
        base_branch: base_branch.to_string(),
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
                head_branch: head_branch.to_string(),
                base_branch: base_branch.to_string(),
            };

            forge.create_pr(req)?
        }
    };

    Ok(pr)
}
