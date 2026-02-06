//! Tests for PR workflow.
//!
//! Tests for:
//! - create_release_prs method
//! - PR existence checking behavior
//! - Creating new PRs vs updating existing ones
//! - Handling empty releasable packages

use super::common::*;
use crate::{
    ReleasaurusError,
    config::{Config, package::PackageConfigBuilder},
    forge::{
        config::PENDING_LABEL,
        request::{Commit, ForgeCommitBuilder, PullRequest},
        traits::MockForge,
    },
};

#[tokio::test]
async fn create_release_prs_succeeds_when_no_commits_since_last_tag() {
    let mut mock_forge = MockForge::new();

    // Has tag, but no new commits
    mock_forge
        .expect_get_latest_tag_for_prefix()
        .returning(|_| {
            Ok(Some(crate::analyzer::release::Tag {
                name: "v1.0.0".to_string(),
                semver: Version::parse("1.0.0").unwrap(),
                sha: "abc123".to_string(),
                timestamp: Some(1234567890),
            }))
        });

    // No commits since tag
    mock_forge.expect_get_commits().returning(|_, _| Ok(vec![]));

    // With no new commits, no PR operations should occur
    mock_forge.expect_get_open_release_pr().times(0);
    mock_forge.expect_create_pr().times(0);
    mock_forge.expect_update_pr().times(0);

    let orchestrator = create_test_orchestrator(mock_forge);

    orchestrator.create_release_prs(None).await.unwrap();
}

#[tokio::test]
async fn create_release_prs_returns_error_when_merged_pr_not_yet_released() {
    let mut mock_forge = MockForge::new();

    // No tags exist yet
    mock_forge
        .expect_get_latest_tag_for_prefix()
        .returning(|_| Ok(None));

    mock_forge.expect_get_commits().returning(|_, _| {
        Ok(vec![
            ForgeCommitBuilder::default()
                .id("abc123")
                .files(vec!["dummy.txt".into()])
                .build()
                .unwrap(),
        ])
    });

    // A merged release PR exists that hasn't been tagged yet
    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| {
            Ok(Some(PullRequest {
                number: 42,
                sha: "def456".into(),
                body: "".into(),
            }))
        })
        .times(1);

    // Should never reach branch creation or PR operations
    mock_forge.expect_create_release_branch().times(0);
    mock_forge.expect_get_open_release_pr().times(0);
    mock_forge.expect_create_pr().times(0);
    mock_forge.expect_update_pr().times(0);
    mock_forge.expect_replace_pr_labels().times(0);

    let orchestrator = create_test_orchestrator(mock_forge);

    let err = orchestrator.create_release_prs(None).await.unwrap_err();

    assert!(matches!(err, ReleasaurusError::PendingRelease { .. }));
}

#[tokio::test]
async fn create_release_prs_creates_new_prs() {
    let mut mock_forge = MockForge::new();

    // No tags exist yet
    mock_forge
        .expect_get_latest_tag_for_prefix()
        .returning(|_| Ok(None));

    mock_forge.expect_get_commits().returning(|_, _| {
        Ok(vec![
            ForgeCommitBuilder::default()
                .id("abc123")
                .files(vec![
                    "packages/pkg-a/dummy.txt".into(),
                    "packages/pkg-b/dummy.txt".into(),
                ])
                .build()
                .unwrap(),
        ])
    });

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    mock_forge
        .expect_get_open_release_pr()
        .returning(|_| Ok(None))
        .times(2);

    mock_forge
        .expect_create_release_branch()
        .returning(|_| {
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        })
        .times(2);

    mock_forge
        .expect_create_pr()
        .returning(|req| {
            let mut pr_number = 1;
            if req.base_branch.contains("pkg-b") {
                pr_number = 2;
            }
            Ok(PullRequest {
                number: pr_number,
                sha: "abc123".into(),
                body: "".into(),
            })
        })
        .times(2);

    mock_forge.expect_update_pr().times(0);

    mock_forge
        .expect_replace_pr_labels()
        .times(2)
        .withf(|req| {
            (req.pr_number == 1 || req.pr_number == 2)
                && req.labels.contains(&PENDING_LABEL.into())
        })
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

    orchestrator.create_release_prs(None).await.unwrap();
}

#[tokio::test]
async fn create_release_prs_targets_specific_package() {
    let mut mock_forge = MockForge::new();

    // No tags exist yet
    mock_forge
        .expect_get_latest_tag_for_prefix()
        .returning(|_| Ok(None));

    mock_forge.expect_get_commits().returning(|_, _| {
        Ok(vec![
            ForgeCommitBuilder::default()
                .id("abc123")
                .files(vec![
                    "packages/pkg-a/dummy.txt".into(),
                    "packages/pkg-b/dummy.txt".into(),
                ])
                .build()
                .unwrap(),
        ])
    });

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    mock_forge
        .expect_get_open_release_pr()
        .returning(|_| Ok(None))
        .times(1);

    mock_forge
        .expect_create_release_branch()
        .returning(|_| {
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        })
        .times(1);

    mock_forge
        .expect_create_pr()
        .withf(|req| req.head_branch.contains("pkg-a"))
        .returning(|_| {
            Ok(PullRequest {
                number: 1,
                sha: "abc123".into(),
                body: "".into(),
            })
        })
        .times(1);

    mock_forge.expect_update_pr().times(0);

    mock_forge
        .expect_replace_pr_labels()
        .times(1)
        .withf(|req| {
            req.pr_number == 1 && req.labels.contains(&PENDING_LABEL.into())
        })
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

    orchestrator
        .create_release_prs(Some("pkg-a".into()))
        .await
        .unwrap();
}

#[tokio::test]
async fn create_release_prs_returns_error_for_invalid_package_name() {
    let mock_forge = MockForge::new();

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
        None,
    );

    let err = orchestrator
        .create_release_prs(Some("nope".into()))
        .await
        .unwrap_err();

    assert!(matches!(err, ReleasaurusError::InvalidArgs(_)))
}

#[tokio::test]
async fn create_release_prs_updates_existing_prs() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_latest_tag_for_prefix()
        .returning(|_| {
            Ok(Some(crate::analyzer::release::Tag {
                name: "v1.0.0".to_string(),
                semver: Version::parse("1.0.0").unwrap(),
                sha: "abc123".to_string(),
                timestamp: Some(100),
            }))
        });

    mock_forge.expect_get_commits().returning(|_, _| {
        Ok(vec![
            ForgeCommitBuilder::default()
                .id("abc123")
                .files(vec!["dummy.txt".into()])
                .timestamp(200)
                .build()
                .unwrap(),
        ])
    });

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    mock_forge
        .expect_get_open_release_pr()
        .returning(|_| {
            Ok(Some(PullRequest {
                number: 1,
                sha: "def123".into(),
                body: "".into(),
            }))
        })
        .times(1);

    mock_forge
        .expect_create_release_branch()
        .returning(|_| {
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        })
        .times(1);

    mock_forge.expect_create_pr().times(0);

    mock_forge.expect_update_pr().returning(|_| Ok(())).times(1);

    mock_forge
        .expect_replace_pr_labels()
        .times(1)
        .withf(|req| {
            req.pr_number == 1 && req.labels.contains(&PENDING_LABEL.into())
        })
        .returning(|_| Ok(()));

    let orchestrator = create_test_orchestrator(mock_forge);

    orchestrator.create_release_prs(None).await.unwrap();
}
