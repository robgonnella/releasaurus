use color_eyre::eyre::Result;
use gitlab::api::Query;
use octocrab::{Octocrab, params};
use reqwest::header::{HeaderMap, HeaderValue};
use secrecy::ExposeSecret;
use serde::Serialize;
use std::{
    fs::OpenOptions, io::Write, path::PathBuf, sync::Once, thread,
    time::Duration,
};
use tempfile::TempDir;

use crate::{
    cli,
    forge::{config::RemoteConfig, traits::Forge, types::ReleasePullRequest},
    repo,
};

#[derive(Debug, Serialize)]
struct MergeData {
    #[serde(rename(serialize = "Do"))]
    pub action: String,
    delete_branch_after_merge: bool,
}

static INIT: Once = Once::new();

fn initialize_logger() {
    INIT.call_once(|| {
        let config = simplelog::ConfigBuilder::new()
            .add_filter_allow_str("releasaurus")
            .build();

        simplelog::TermLogger::init(
            simplelog::LevelFilter::Debug,
            config,
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        )
        .unwrap()
    });
}

pub fn init(
    args: &cli::Args,
) -> Result<(Box<dyn Forge>, repo::Repository, TempDir)> {
    initialize_logger();
    let tmp = TempDir::new()?;
    let remote = args.get_remote()?;
    let forge = remote.get_forge()?;
    let config = forge.config().to_owned();
    let repository = repo::Repository::new(tmp.path(), config)?;
    Ok((forge, repository, tmp))
}

pub fn overwrite_file(file_path: PathBuf, content: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(file_path)?;

    file.write_all(content.as_bytes())?;

    Ok(())
}

pub fn merge_github_release_pr(
    pr: ReleasePullRequest,
    config: &RemoteConfig,
) -> Result<()> {
    // pause thread to make sure updates have registered and branch is mergeable
    thread::sleep(Duration::from_millis(2000));

    let base_uri = format!("{}://api.{}", config.scheme, config.host);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let builder = Octocrab::builder()
            .personal_token(config.token.clone())
            .base_uri(base_uri.clone())
            .unwrap();

        let octocrab = builder.build().unwrap();

        let handler = octocrab.pulls(&config.owner, &config.repo);

        handler
            .merge(pr.number)
            .method(params::pulls::MergeMethod::Rebase)
            .send()
            .await
            .unwrap();
    });

    Ok(())
}

pub fn merge_gitea_release_pr(
    pr: ReleasePullRequest,
    config: &RemoteConfig,
) -> Result<()> {
    // pause thread to make sure updates have registered and branch is mergeable
    thread::sleep(Duration::from_millis(2000));

    let token = config.token.expose_secret();

    let mut headers = HeaderMap::new();

    let token_value =
        HeaderValue::from_str(format!("token {}", token).as_str())?;

    headers.append("Authorization", token_value);

    let client = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()?;

    let data = MergeData {
        action: "fast-forward-only".into(),
        delete_branch_after_merge: true,
    };

    let merge_url = format!(
        "{}://{}/api/v1/repos/{}/pulls/{}/merge",
        config.scheme, config.host, config.path, pr.number
    );

    let response = client.post(&merge_url).json(&data).send()?;
    response.error_for_status()?;

    Ok(())
}

pub fn merge_gitlab_release_pr(
    pr: ReleasePullRequest,
    config: &RemoteConfig,
) -> Result<()> {
    // pause thread to make sure updates have registered and branch is mergeable
    thread::sleep(Duration::from_millis(2000));
    let project_id = config.path.clone();

    let token = config.token.expose_secret();

    let gl = gitlab::GitlabBuilder::new(config.host.clone(), token).build()?;

    let endpoint =
        gitlab::api::projects::merge_requests::MergeMergeRequest::builder()
            .project(project_id)
            .merge_request(pr.number)
            .squash(true)
            .build()?;

    gitlab::api::ignore(endpoint).query(&gl)?;

    Ok(())
}
