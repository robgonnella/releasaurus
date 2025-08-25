use color_eyre::eyre::Result;
use octocrab::{Octocrab, params};
use releasaurus_core::forge::{
    config::{PENDING_LABEL, RemoteConfig},
    github::Github,
    traits::Forge,
    types::{CreatePrRequest, GetPrRequest, PrLabelsRequest, UpdatePrRequest},
};
use secrecy::SecretString;
use std::env;

fn delete_label(config: &RemoteConfig, label: String) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let builder = Octocrab::builder()
            .personal_token(config.token.clone())
            .base_uri(format!("{}://api.{}", config.scheme, config.host))?;

        let octocrab = builder.build()?;

        octocrab
            .issues(&config.owner, &config.repo)
            .delete_label(label)
            .await?;

        Ok(())
    })
}

fn close_pr(config: &RemoteConfig, pr_number: u64) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let builder = Octocrab::builder()
            .personal_token(config.token.clone())
            .base_uri(format!("{}://api.{}", config.scheme, config.host))?;

        let octocrab = builder.build()?;

        octocrab
            .pulls(&config.owner, &config.repo)
            .update(pr_number)
            .state(params::pulls::State::Closed)
            .send()
            .await?;

        Ok(())
    })
}

#[test]
fn test_github_forge() {
    let result = env::var("GH_TEST_TOKEN");
    assert!(
        result.is_ok(),
        "must set GH_TEST_TOKEN as environment variable to run these tests"
    );

    let token = result.unwrap();

    let remote_config = RemoteConfig {
        scheme: "https".into(),
        host: "github.com".into(),
        owner: "robgonnella".into(),
        repo: "test-repo".into(),
        token: SecretString::from(token),
        commit_link_base_url: "".into(),
        release_link_base_url: "".into(),
    };

    let result = Github::new(remote_config.clone());
    assert!(result.is_ok(), "failed to create github forge");
    let forge = result.unwrap();

    let req = CreatePrRequest {
        head_branch: "test-branch".into(),
        base_branch: "main".into(),
        body: "super duper!".into(),
        title: "The is my test PR".into(),
    };

    let result = forge.create_pr(req);
    assert!(result.is_ok(), "failed to create PR");
    let pr_number = result.unwrap();

    let req = UpdatePrRequest {
        pr_number,
        body: "now this is a good body!".into(),
    };

    let result = forge.update_pr(req);
    assert!(result.is_ok(), "failed to update PR");

    let new_label = "releasaurus:1".to_string();

    let req = PrLabelsRequest {
        pr_number,
        labels: vec![new_label.clone(), PENDING_LABEL.into()],
    };

    let result = forge.replace_pr_labels(req);
    assert!(result.is_ok(), "failed to replace PR labels");

    let req = GetPrRequest {
        head_branch: "test-branch".into(),
        base_branch: "main".into(),
    };
    let result = forge.get_pr_number(req);
    assert!(result.is_ok(), "failed to get PR number");

    let result = close_pr(&remote_config, pr_number);
    assert!(result.is_ok(), "failed to close PR");

    let result = delete_label(&remote_config, new_label);
    assert!(result.is_ok(), "failed to delete label")
}
