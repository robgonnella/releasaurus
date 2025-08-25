use color_eyre::eyre::Result;
use gitlab::{
    Gitlab as GitlabClient,
    api::{
        Query, ignore,
        projects::{
            labels::DeleteLabel,
            merge_requests::{EditMergeRequest, MergeRequestStateEvent},
        },
    },
};
use releasaurus_core::forge::{
    config::{PENDING_LABEL, RemoteConfig},
    gitlab::Gitlab,
    traits::Forge,
    types::{CreatePrRequest, GetPrRequest, PrLabelsRequest, UpdatePrRequest},
};
use secrecy::{ExposeSecret, SecretString};
use std::env;

fn delete_label(
    gl: &GitlabClient,
    project_id: String,
    label: String,
) -> Result<()> {
    let endpoint = DeleteLabel::builder()
        .project(project_id)
        .label(label)
        .build()?;
    ignore(endpoint).query(gl)?;
    Ok(())
}

fn close_pr(
    gl: &GitlabClient,
    project_id: String,
    pr_number: u64,
) -> Result<()> {
    let endpoint = EditMergeRequest::builder()
        .project(project_id)
        .merge_request(pr_number)
        .state_event(MergeRequestStateEvent::Close)
        .build()?;

    ignore(endpoint).query(gl)?;

    Ok(())
}

#[test]
fn test_gitlab_forge() {
    let result = env::var("GL_TEST_TOKEN");
    assert!(
        result.is_ok(),
        "must set GL_TEST_TOKEN as environment variable to run these tests"
    );

    let token = result.unwrap();

    let remote_config = RemoteConfig {
        scheme: "https".into(),
        host: "gitlab.com".into(),
        owner: "rgon".into(),
        repo: "test-repo".into(),
        token: SecretString::from(token),
        commit_link_base_url: "".into(),
        release_link_base_url: "".into(),
    };

    let result = gitlab::GitlabBuilder::new(
        remote_config.host.clone(),
        remote_config.token.expose_secret(),
    )
    .build();
    assert!(result.is_ok(), "failed to create gitlab client");
    let gl = result.unwrap();
    let project_id = format!("{}/{}", remote_config.owner, remote_config.repo);

    let result = Gitlab::new(remote_config.clone());
    assert!(result.is_ok(), "failed to create gitlab forge");
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

    let result = close_pr(&gl, project_id.clone(), pr_number);
    assert!(result.is_ok(), "failed to close PR");

    let result = delete_label(&gl, project_id, new_label);
    assert!(result.is_ok(), "failed to delete label")
}
