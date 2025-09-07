//! Defines execution function for the release command
use log::*;

use crate::{
    analyzer::{cliff::CliffAnalyzer, types::ProjectedRelease},
    cli,
    command::common,
    config,
    forge::{config::TAGGED_LABEL, traits::Forge, types::ReleasePullRequest},
    repo::Repository,
    result::Result,
};

pub fn execute(args: &cli::Args) -> Result<()> {
    let remote = args.get_remote()?;
    let forge = remote.get_forge()?;
    let (repo, tmp_dir) = common::setup_repository(forge.as_ref())?;

    let merged_pr = forge.get_merged_release_pr()?;

    if merged_pr.is_none() {
        warn!("releases are up-to-date: nothing to release");
        return Ok(());
    }

    let merged_pr = merged_pr.unwrap();

    let cli_config = common::load_configuration(tmp_dir.path())?;

    let releases = process_packages_for_release(
        &repo,
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
    merged_pr: &ReleasePullRequest,
    cli_config: &config::CliConfig,
    remote_config: &crate::forge::config::RemoteConfig,
) -> Result<Vec<ProjectedRelease>> {
    let mut releases = vec![];

    for package in &cli_config.packages {
        if let Some(release) = create_package_release(
            repo,
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
    merged_pr: &ReleasePullRequest,
    package: &config::CliPackageConfig,
    cli_config: &config::CliConfig,
    remote_config: &crate::forge::config::RemoteConfig,
) -> Result<Option<ProjectedRelease>> {
    let tag_prefix = common::get_tag_prefix(package);
    let starting_sha = repo.get_latest_tagged_starting_point(&tag_prefix)?;

    let changelog_config = common::create_changelog_config(
        package,
        cli_config,
        remote_config,
        starting_sha,
        String::from(repo.workdir_as_str()),
    );

    let analyzer = CliffAnalyzer::new(changelog_config)?;
    let output = analyzer.process_repository()?;

    if let Some(next_version) = output.next_version
        && let Some(projected_release) = output.projected_release
    {
        repo.tag_commit(&next_version.tag, &merged_pr.sha)?;
        Ok(Some(projected_release))
    } else {
        Ok(None)
    }
}

fn publish_releases(
    forge: &dyn Forge,
    repo: &Repository,
    releases: &[ProjectedRelease],
) -> Result<()> {
    for release in releases {
        publish_single_release(forge, repo, release)?;
    }
    Ok(())
}

fn publish_single_release(
    forge: &dyn Forge,
    repo: &Repository,
    release: &ProjectedRelease,
) -> Result<()> {
    info!("processing releasable package");
    info!("release path: {}", release.path);
    info!("release tag: {}", release.tag);
    info!("release sha: {}", release.sha);

    info!("pushing tag: {}", release.tag);
    repo.push_tag_to_default_branch(&release.tag)?;

    info!("creating release: {}", release.tag);
    forge.create_release(&release.tag, &release.sha, &release.notes)?;

    // TODO: comment on PR about release

    Ok(())
}
