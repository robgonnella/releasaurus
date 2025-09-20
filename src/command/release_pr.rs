//! Release pull request creation command implementation.

use log::*;
use std::collections::HashMap;

use crate::{
    analyzer::{changelog::Analyzer, types::Release},
    cli,
    command::common,
    config,
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, PENDING_LABEL},
        traits::Forge,
        types::{CreatePrRequest, GetPrRequest, UpdatePrRequest},
    },
    repo::Repository,
    result::Result,
    updater::manager::UpdaterManager,
};

/// Content for release pull request.
struct PrContent {
    pub title: String,
    pub body: Vec<String>,
    pub releasable: bool,
}

/// Execute release-pr command to analyze commits and create release pull request.
pub fn execute(args: &cli::Args) -> Result<()> {
    let remote = args.get_remote()?;
    let forge = remote.get_forge()?;

    let (repo, tmp_dir) =
        common::setup_repository(args.clone_depth, forge.as_ref())?;
    let cli_config = common::load_configuration(tmp_dir.path())?;

    let release_branch =
        common::setup_release_branch(&repo, DEFAULT_PR_BRANCH_PREFIX)?;

    let manifest =
        process_packages(&repo, forge.as_ref(), &cli_config, forge.config())?;

    info!("manifest: {:#?}", manifest);

    let pr_content = create_pr_content(manifest, &repo)?;

    if !pr_content.releasable {
        info!("no releasable commits found");
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
    forge: &dyn Forge,
    cli_config: &config::CliConfig,
    remote_config: &crate::forge::config::RemoteConfig,
) -> Result<HashMap<String, Release>> {
    let mut manifest: HashMap<String, Release> = HashMap::new();

    for package in &cli_config.packages {
        if let Some(release) = process_single_package(
            package,
            repo,
            forge,
            cli_config,
            remote_config,
        )? {
            manifest.insert(package.path.clone(), release);
        }
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
    forge: &dyn Forge,
    cli_config: &config::CliConfig,
    remote_config: &crate::forge::config::RemoteConfig,
) -> Result<Option<Release>> {
    let tag_prefix = common::get_tag_prefix(package);

    common::log_package_processing(&package.path, &tag_prefix);

    let starting_tag = forge.get_latest_tag_for_prefix(&tag_prefix)?;

    let changelog_config = common::create_changelog_config(
        package,
        cli_config,
        remote_config,
        starting_tag,
        String::from(repo.workdir_as_str()),
    );

    let analyzer = Analyzer::new(changelog_config, repo)?;
    analyzer.write_changelog()
}

fn create_pr_content(
    manifest: HashMap<String, Release>,
    repo: &Repository,
) -> Result<PrContent> {
    let mut title = format!("chore({}): release", repo.default_branch);
    let mut include_title_version = false;
    let mut body = vec![];
    let releasable = !manifest.is_empty();

    let mut start_tag = "<details>";

    // auto-open dropdown if there's only one package
    if manifest.len() == 1 {
        include_title_version = true;
        start_tag = "<details open>";
    }

    for (name, release) in manifest {
        if release.tag.is_none() {
            warn!("no projected tag found fo release: {:#?}", release);
            continue;
        }

        let tag = release.tag.unwrap();

        debug!("\n\n{}\n  next_version: {:?}", name, tag);

        // only keep packages that have releasable commits

        info!("{name} is releasable: next version: {}", tag.semver);

        // create the drop down
        let drop_down = format!(
            "{start_tag}<summary>{}</summary>\n\n{}</details>",
            tag.semver, release.notes
        );

        body.push(drop_down);

        if include_title_version {
            title = format!("{} {}", title, tag.semver)
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
