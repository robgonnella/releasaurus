//! Final release publication and tagging command implementation.
use log::*;

use crate::{
    analyzer::{changelog::Analyzer, types::Release},
    cli,
    command::common,
    config,
    forge::{config::TAGGED_LABEL, traits::Forge, types::ReleasePullRequest},
    repo::Repository,
    result::Result,
};

/// Execute release command to create git tags and publish final release.
pub fn execute(args: &cli::Args) -> Result<()> {
    let remote = args.get_remote()?;
    let forge = remote.get_forge()?;
    let (repo, tmp_dir) =
        common::setup_repository(args.clone_depth, forge.as_ref())?;

    let merged_pr = forge.get_merged_release_pr()?;

    if merged_pr.is_none() {
        warn!("releases are up-to-date: nothing to release");
        return Ok(());
    }

    let merged_pr = merged_pr.unwrap();

    let cli_config = common::load_configuration(tmp_dir.path())?;

    let releases = process_packages_for_release(
        &repo,
        forge.as_ref(),
        &merged_pr,
        &cli_config,
        forge.config(),
    )?;

    if releases.is_empty() {
        info!("releases are all up-to-date: nothing to do");
        return Ok(());
    }

    publish_releases(forge.as_ref(), &repo, &releases)?;

    common::update_pr_labels(
        forge.as_ref(),
        merged_pr.number,
        vec![TAGGED_LABEL.into()],
    )?;

    // Keep tmp_dir alive until the end to prevent cleanup
    drop(tmp_dir);

    Ok(())
}

fn process_packages_for_release(
    repo: &Repository,
    forge: &dyn Forge,
    merged_pr: &ReleasePullRequest,
    cli_config: &config::CliConfig,
    remote_config: &crate::forge::config::RemoteConfig,
) -> Result<Vec<Release>> {
    let mut releases = vec![];

    for package in &cli_config.packages {
        if let Some(release) = create_package_release(
            repo,
            forge,
            merged_pr,
            package,
            cli_config,
            remote_config,
        )? {
            releases.push(release);
        }
    }

    Ok(releases)
}

fn create_package_release(
    repo: &Repository,
    forge: &dyn Forge,
    merged_pr: &ReleasePullRequest,
    package: &config::CliPackageConfig,
    cli_config: &config::CliConfig,
    remote_config: &crate::forge::config::RemoteConfig,
) -> Result<Option<Release>> {
    let tag_prefix = common::get_tag_prefix(package);
    let starting_tag = forge.get_latest_tag_for_prefix(&tag_prefix)?;

    let changelog_config = common::create_changelog_config(
        package,
        cli_config,
        remote_config,
        starting_tag,
        String::from(repo.workdir_as_str()),
    );

    let analyzer = Analyzer::new(changelog_config, repo)?;
    let release = analyzer.process_repository()?;

    if let Some(release) = release
        && let Some(tag) = release.tag.clone()
    {
        repo.tag_commit(&tag.name, &merged_pr.sha)?;
        Ok(Some(release))
    } else {
        Ok(None)
    }
}

fn publish_releases(
    forge: &dyn Forge,
    repo: &Repository,
    releases: &[Release],
) -> Result<()> {
    for release in releases {
        publish_single_release(forge, repo, release)?;
    }
    Ok(())
}

fn publish_single_release(
    forge: &dyn Forge,
    repo: &Repository,
    release: &Release,
) -> Result<()> {
    if let Some(tag) = release.tag.clone() {
        info!("processing releasable package");
        info!("release tag: {}", tag);

        info!("pushing tag: {}", tag);
        repo.push_tag_to_default_branch(&tag.name)?;

        info!("creating release: {}", tag);
        forge.create_release(&tag.name, &release.sha, &release.notes)?;
    }

    // TODO: comment on PR about release

    Ok(())
}
