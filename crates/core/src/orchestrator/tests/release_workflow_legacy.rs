//! Tests for release workflow with the legacy PR body format.
//!
//! Tests for:
//! - Finding merged PRs using legacy `<!--metadata--><details>` body layout
//! - Auto-start next release behavior with legacy body
//! - Handling separate pull requests with legacy body
//! - Targeting a specific package with legacy body

use crate::{
    config::{Config, package::PackageConfigBuilder},
    forge::{
        config::TAGGED_LABEL,
        request::{Commit, GetPrRequest, PullRequest, Tag},
        traits::MockForge,
    },
};

use super::common::*;

#[tokio::test]
async fn create_releases_finds_merged_pr_and_creates_release() {
    let pr_body = format!(
        r#"
<!--{{"metadata":{{"name":"{TEST_PKG_NAME}","tag":"v1.0.0","notes":"Release notes"}}}}-->
<details><summary>v1.0.0</summary>
Release notes
</details>
"#
    );

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .times(1)
        .returning(move |_| {
            Ok(Some(PullRequest {
                number: 123,
                sha: "abc123".to_string(),
                body: pr_body.clone(),
            }))
        });

    mock_forge
        .expect_tag_commit()
        .times(1)
        .withf(|tag, sha| tag == "v1.0.0" && sha == "abc123")
        .returning(|_, _| Ok(()));

    mock_forge
        .expect_create_release()
        .times(1)
        .withf(|tag, sha, _| tag == "v1.0.0" && sha == "abc123")
        .returning(|_, _, _| Ok(()));

    mock_forge
        .expect_replace_pr_labels()
        .times(1)
        .withf(|req| {
            req.pr_number == 123 && req.labels.contains(&TAGGED_LABEL.into())
        })
        .returning(|_| Ok(()));

    let orchestrator = create_test_orchestrator_with_config(
        mock_forge,
        vec![
            PackageConfigBuilder::default()
                .name(TEST_PKG_NAME)
                .path(".")
                .build()
                .unwrap(),
        ],
        None,
    );

    orchestrator.create_releases(None).await.unwrap();
}

#[tokio::test]
async fn create_releases_triggers_auto_start_next() {
    let pr_body = format!(
        r#"
<!--{{"metadata":{{"name":"{TEST_PKG_NAME}","tag":"v1.0.0","notes":"Release notes"}}}}-->
<details><summary>v1.0.0</summary>
Release notes
</details>
"#
    );

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .times(1)
        .returning(move |_| {
            Ok(Some(PullRequest {
                number: 123,
                sha: "abc123".to_string(),
                body: pr_body.clone(),
            }))
        });

    mock_forge.expect_tag_commit().returning(|_, _| Ok(()));
    mock_forge
        .expect_create_release()
        .returning(|_, _, _| Ok(()));
    mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

    mock_forge
        .expect_get_latest_tags_for_prefix()
        .returning(|_, _| {
            Ok(vec![Tag {
                semver: Version::parse("1.0.0").unwrap(),
                ..Default::default()
            }])
        });

    mock_forge.expect_get_commits().returning(|_, _| Ok(vec![]));

    mock_forge.expect_create_commit().times(1).returning(|_| {
        Ok(Commit {
            sha: "new-commit".to_string(),
        })
    });

    let orchestrator = create_test_orchestrator_with_config(
        mock_forge,
        vec![
            PackageConfigBuilder::default()
                .name(TEST_PKG_NAME)
                .path(".")
                .auto_start_next(true)
                .build()
                .unwrap(),
        ],
        None,
    );

    orchestrator.create_releases(None).await.unwrap();
}

#[tokio::test]
async fn create_releases_handles_separate_pull_requests() {
    let pr_body_a = r#"
<!--{"metadata":{"name":"pkg-a","tag":"v1.0.0","notes":"Release A"}}-->
<details><summary>v1.0.0</summary>
Release A
</details>
"#
    .to_string();

    let pr_body_b = r#"
<!--{"metadata":{"name":"pkg-b","tag":"v2.0.0","notes":"Release B"}}-->
<details><summary>v2.0.0</summary>
Release B
</details>
"#
    .to_string();

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .times(2)
        .returning(move |req: GetPrRequest| {
            if req.head_branch.contains("pkg-a") {
                Ok(Some(PullRequest {
                    number: 123,
                    sha: "sha-a".to_string(),
                    body: pr_body_a.clone(),
                }))
            } else {
                Ok(Some(PullRequest {
                    number: 124,
                    sha: "sha-b".to_string(),
                    body: pr_body_b.clone(),
                }))
            }
        });

    mock_forge
        .expect_tag_commit()
        .times(2)
        .returning(|_, _| Ok(()));
    mock_forge
        .expect_create_release()
        .times(2)
        .returning(|_, _, _| Ok(()));
    mock_forge
        .expect_replace_pr_labels()
        .times(2)
        .returning(|_| Ok(()));

    let config = Config {
        separate_pull_requests: true,
        ..Default::default()
    };

    let orchestrator = create_test_orchestrator_with_config(
        mock_forge,
        vec![
            PackageConfigBuilder::default()
                .name("pkg-a")
                .path("packages/pkg-a")
                .build()
                .unwrap(),
            PackageConfigBuilder::default()
                .name("pkg-b")
                .path("packages/pkg-b")
                .build()
                .unwrap(),
        ],
        Some(config),
    );

    orchestrator.create_releases(None).await.unwrap();
}

#[tokio::test]
async fn create_releases_targets_specific_package() {
    let pr_body_a = r#"
<!--{"metadata":{"name":"pkg-a","tag":"pkg-a-v1.0.0","notes":"Release A"}}-->
<details><summary>v1.0.0</summary>
Release A
</details>
"#
    .to_string();

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .times(1)
        .returning(move |_| {
            Ok(Some(PullRequest {
                number: 123,
                sha: "sha-a".to_string(),
                body: pr_body_a.clone(),
            }))
        });

    mock_forge
        .expect_tag_commit()
        .withf(|tag_name, _| tag_name.contains("pkg-a"))
        .times(1)
        .returning(|_, _| Ok(()));
    mock_forge
        .expect_create_release()
        .withf(|tag, _, _| tag.contains("pkg-a"))
        .times(1)
        .returning(|_, _, _| Ok(()));
    mock_forge
        .expect_replace_pr_labels()
        .times(1)
        .returning(|_| Ok(()));

    let config = Config {
        separate_pull_requests: true,
        ..Default::default()
    };

    let orchestrator = create_test_orchestrator_with_config(
        mock_forge,
        vec![
            PackageConfigBuilder::default()
                .name("pkg-a")
                .path("packages/pkg-a")
                .tag_prefix("pkg-a-v")
                .build()
                .unwrap(),
            PackageConfigBuilder::default()
                .name("pkg-b")
                .path("packages/pkg-b")
                .tag_prefix("pkg-b-v")
                .build()
                .unwrap(),
        ],
        Some(config),
    );

    orchestrator
        .create_releases(Some("pkg-a".into()))
        .await
        .unwrap();
}
