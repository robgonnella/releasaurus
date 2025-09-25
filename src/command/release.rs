//! Final release publication and tagging command implementation.
use log::*;

use crate::{
    analyzer::release::Release,
    cli,
    command::common,
    config,
    forge::{config::TAGGED_LABEL, request::PullRequest, traits::Forge},
    result::Result,
};

/// Execute release command to create git tags and publish final release.
pub async fn execute(args: &cli::Args) -> Result<()> {
    let remote = args.get_remote()?;
    let forge = remote.get_forge().await?;
    let merged_pr = forge.get_merged_release_pr().await?;

    if merged_pr.is_none() {
        warn!("releases are up-to-date: nothing to release");
        return Ok(());
    }

    let merged_pr = merged_pr.unwrap();

    let config = forge.load_config().await?;

    let releases =
        process_packages_for_release(forge.as_ref(), &merged_pr, &config)?;

    if releases.is_empty() {
        info!("releases are all up-to-date: nothing to do");
        return Ok(());
    }

    publish_releases(forge.as_ref(), &releases).await?;

    common::update_pr_labels(
        forge.as_ref(),
        merged_pr.number,
        vec![TAGGED_LABEL.into()],
    )
    .await?;

    Ok(())
}

fn process_packages_for_release(
    forge: &dyn Forge,
    merged_pr: &PullRequest,
    conf: &config::Config,
) -> Result<Vec<Release>> {
    let mut releases = vec![];

    for package in &conf.packages {
        if let Some(release) =
            create_package_release(forge, merged_pr, package, conf)?
        {
            releases.push(release);
        }
    }

    Ok(releases)
}

fn create_package_release(
    _forge: &dyn Forge,
    _merged_pr: &PullRequest,
    _package: &config::PackageConfig,
    _conf: &config::Config,
) -> Result<Option<Release>> {
    // let tag_prefix = common::get_tag_prefix(package);
    // let starting_tag = forge.get_latest_tag_for_prefix(&tag_prefix)?;

    // let changelog_config = common::create_changelog_config(
    //     package,
    //     cli_config,
    //     remote_config,
    //     starting_tag,
    //     String::from(repo.workdir_as_str()),
    // );

    // let analyzer = Analyzer::new(changelog_config, repo)?;
    // let release = analyzer.process_repository()?;

    // if let Some(release) = release
    //     && let Some(tag) = release.tag.clone()
    // {
    //     repo.tag_commit(&tag.name, &merged_pr.sha)?;
    //     Ok(Some(release))
    // } else {
    //     Ok(None)
    // }
    Ok(None)
}

async fn publish_releases(
    forge: &dyn Forge,
    releases: &[Release],
) -> Result<()> {
    for release in releases {
        publish_single_release(forge, release).await?;
    }
    Ok(())
}

async fn publish_single_release(
    forge: &dyn Forge,
    release: &Release,
) -> Result<()> {
    if let Some(tag) = release.tag.clone() {
        info!("processing releasable package");
        info!("release tag: {}", tag);

        info!("pushing tag: {}", tag);
        forge.tag_commit(&tag.name, &release.sha).await?;

        info!("creating release: {}", tag);
        forge
            .create_release(&tag.name, &release.sha, &release.notes)
            .await?;
    }

    // TODO: comment on PR about release

    Ok(())
}
