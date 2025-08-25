use color_eyre::eyre::Result;
use releasaurus_core::forge::{
    config::{PENDING_LABEL, RemoteConfig},
    gitea::{Gitea, Label},
    traits::Forge,
    types::{CreatePrRequest, GetPrRequest, PrLabelsRequest, UpdatePrRequest},
};
use reqwest::{
    Url,
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};
use secrecy::SecretString;
use std::{collections::HashMap, env};

fn delete_label(
    client: &Client,
    base_url: &Url,
    label_name: String,
) -> Result<()> {
    let labels_url = base_url.join("labels")?;
    let request = client.get(labels_url).build()?;
    let response = client.execute(request)?;
    let result = response.error_for_status()?;
    let labels: Vec<Label> = result.json()?;

    for label in labels {
        if label.name == label_name {
            let label_path = format!("labels/{}", label.id);
            let delete_url = base_url.join(label_path.as_str())?;
            let request = client.delete(delete_url).build()?;
            let response = client.execute(request)?;
            response.error_for_status()?;
            return Ok(());
        }
    }

    Ok(())
}

fn close_pr(client: &Client, base_url: &Url, pr_number: u64) -> Result<()> {
    let mut body = HashMap::new();
    body.insert("state", "closed");
    let pulls_url = base_url.join(format!("pulls/{}", pr_number).as_str())?;
    let request = client.patch(pulls_url).json(&body).build()?;
    let response = client.execute(request)?;
    response.error_for_status()?;
    Ok(())
}

#[test]
fn test_gitea_forge() {
    let result = env::var("GT_TEST_TOKEN");
    assert!(
        result.is_ok(),
        "must set GT_TEST_TOKEN as environment variable to run these tests"
    );

    let token = result.unwrap();

    let remote_config = RemoteConfig {
        scheme: "https".into(),
        host: "gitea.com".into(),
        owner: "rgon".into(),
        repo: "test-repo".into(),
        token: SecretString::from(token.clone()),
        commit_link_base_url: "".into(),
        release_link_base_url: "".into(),
    };

    let mut headers = HeaderMap::new();

    let token_value =
        HeaderValue::from_str(format!("token {}", token).as_str()).unwrap();

    headers.append("Authorization", token_value);

    let client = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    let base_url = Url::parse(
        format!(
            "{}://{}/api/v1/repos/{}/{}/",
            remote_config.scheme,
            remote_config.host,
            remote_config.owner,
            remote_config.repo
        )
        .as_str(),
    )
    .unwrap();

    let result = Gitea::new(remote_config.clone());
    assert!(result.is_ok(), "failed to create gitea forge");
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

    let result = close_pr(&client, &base_url, pr_number);
    assert!(result.is_ok(), "failed to close PR");

    let result = delete_label(&client, &base_url, new_label);
    assert!(result.is_ok(), "failed to delete label")
}
