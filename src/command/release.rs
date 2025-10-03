//! Final release publication and tagging command implementation.
use log::*;

use crate::{
    analyzer::{Analyzer, config::AnalyzerConfig},
    command::common,
    config,
    forge::{
        config::{RemoteConfig, TAGGED_LABEL},
        request::{PrLabelsRequest, PullRequest},
        traits::Forge,
    },
    result::Result,
};

/// Execute release command to create git tags and publish final release.
pub async fn execute(forge: Box<dyn Forge>) -> Result<()> {
    let remote_config = forge.remote_config();
    let merged_pr = forge.get_merged_release_pr().await?;

    if merged_pr.is_none() {
        warn!("releases are up-to-date: nothing to release");
        return Ok(());
    }

    let merged_pr = merged_pr.unwrap();

    let config = forge.load_config().await?;

    process_packages_for_release(
        forge.as_ref(),
        &remote_config,
        &merged_pr,
        &config,
    )
    .await?;

    let req = PrLabelsRequest {
        pr_number: merged_pr.number,
        labels: vec![TAGGED_LABEL.into()],
    };
    forge.replace_pr_labels(req).await?;

    Ok(())
}

async fn process_packages_for_release(
    forge: &dyn Forge,
    remote_config: &RemoteConfig,
    merged_pr: &PullRequest,
    conf: &config::Config,
) -> Result<()> {
    for package in &conf.packages {
        create_package_release(forge, remote_config, merged_pr, package, conf)
            .await?
    }

    Ok(())
}

async fn create_package_release(
    forge: &dyn Forge,
    remote_config: &RemoteConfig,
    merged_pr: &PullRequest,
    package: &config::PackageConfig,
    config: &config::Config,
) -> Result<()> {
    let tag_prefix = common::get_tag_prefix(package);
    let current_tag = forge.get_latest_tag_for_prefix(&tag_prefix).await?;
    let current_sha = current_tag.clone().map(|t| t.sha);
    let commits = forge.get_commits(&package.path, current_sha).await?;

    let analyzer_config = AnalyzerConfig {
        body: config.changelog.body.clone(),
        release_link_base_url: remote_config.release_link_base_url.clone(),
        tag_prefix: Some(tag_prefix),
    };

    let analyzer = Analyzer::new(analyzer_config)?;
    let release = analyzer.analyze(commits, current_tag)?;

    if let Some(release) = release
        && let Some(tag) = release.tag.clone()
    {
        forge.tag_commit(&tag.name, &merged_pr.sha).await?;
        forge
            .create_release(&tag.name, &release.sha, &release.notes)
            .await?;
    }

    Ok(())
}
