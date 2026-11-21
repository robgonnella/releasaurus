//! Tests for PR workflow.
//!
//! Tests for:
//! - create_release_prs method
//! - PR existence checking behavior
//! - Creating new PRs vs updating existing ones
//! - Handling empty releasable packages

use crate::{
    config::{
        Config, package::PackageConfigBuilder, repository::RepositoryConfig,
    },
    forge::{
        config::PENDING_LABEL,
        request::{
            Commit, ForgeCommitBuilder, PrMetadataBlock, PullRequest, Tag,
            TagVersion,
        },
        traits::MockForge,
    },
    result::ReleasaurusError,
};

use std::sync::{Arc, Mutex};

use super::common::*;

#[tokio::test]
async fn create_release_prs_succeeds_when_no_commits_since_last_tag() {
    let mut mock_forge = MockForge::new();

    // Has tag, but no new commits
    mock_forge
        .expect_get_latest_tags_for_prefix()
        .returning(|_, _, _, _| {
            Ok(vec![Tag {
                name: "v1.0.0".to_string(),
                version: TagVersion::Semantic(Version::parse("1.0.0").unwrap()),
                sha: "abc123".to_string(),
                timestamp: Some(1234567890),
            }])
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
        .expect_get_latest_tags_for_prefix()
        .returning(|_, _, _, _| Ok(vec![]));

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

    // the guard runs before any PR lookup or branch creation
    mock_forge.expect_get_open_release_pr().times(0);
    mock_forge.expect_create_release_branch().times(0);
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
        .expect_get_latest_tags_for_prefix()
        .returning(|_, _, _, _| Ok(vec![]));

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

    mock_forge.expect_get_file_content().returning(|_| Ok(None));

    mock_forge
        .expect_create_release_branch()
        .returning(|_| {
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        })
        .times(2);

    mock_forge.expect_max_pr_body_len().returning(|| None);

    mock_forge
        .expect_encode_pr_metadata()
        .times(2)
        .returning(|json| PrMetadataBlock {
            inline_content: format!("<!--{json}-->"),
            div_attribute: String::new(),
        });

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
        repository: RepositoryConfig {
            separate_pull_requests: true,
            ..RepositoryConfig::default()
        },
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

/// The whole PR-side fallback: forge commits go through the analyzer and
/// the releasable package into a release PR body that is over the forge's
/// limit, so it carries short notes built from those commits while the
/// changelog on the release branch keeps the full notes.
#[tokio::test]
async fn create_release_prs_falls_back_to_short_notes_over_forge_limit() {
    const LIMIT: usize = 4000;

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_latest_tags_for_prefix()
        .returning(|_, _, _, _| Ok(vec![]));

    mock_forge.expect_get_commits().returning(|_, _| {
        let breaking = ForgeCommitBuilder::default()
            .id("breaking")
            .short_id("brk")
            .link("https://example.com/commit/brk")
            .message(
                "feat!: drop the v1 endpoints\n\n\
                 The v1 endpoints are gone; migrate to v2.",
            )
            .timestamp(1_000_001)
            .files(vec!["dummy.txt".into()])
            .build()
            .unwrap();
        let fixes = (0..60).map(|index| {
            ForgeCommitBuilder::default()
                .id(format!("commit{index:03}"))
                .short_id(format!("c{index:03}"))
                .link(format!("https://example.com/commit/c{index:03}"))
                .message(format!(
                    "fix: correct a fairly long commit subject number {index}"
                ))
                .timestamp(1_000_000 - index)
                .files(vec!["dummy.txt".into()])
                .build()
                .unwrap()
        });
        Ok(std::iter::once(breaking).chain(fixes).collect())
    });

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));
    mock_forge
        .expect_get_open_release_pr()
        .returning(|_| Ok(None));
    mock_forge.expect_get_file_content().returning(|_| Ok(None));

    let changelog = Arc::new(Mutex::new(String::new()));
    let branch_changelog = Arc::clone(&changelog);
    mock_forge
        .expect_create_release_branch()
        .times(1)
        .returning(move |req| {
            let change = req
                .file_changes
                .iter()
                .find(|change| change.repo_path.ends_with("CHANGELOG.md"))
                .unwrap();
            *branch_changelog.lock().unwrap() = change.full_content.clone();
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        });

    mock_forge
        .expect_max_pr_body_len()
        .returning(|| Some(LIMIT));
    mock_forge
        .expect_encode_pr_metadata()
        .returning(|json| PrMetadataBlock {
            inline_content: format!("<!--{json}-->"),
            div_attribute: String::new(),
        });

    let body = Arc::new(Mutex::new(String::new()));
    let pr_body = Arc::clone(&body);
    mock_forge
        .expect_create_pr()
        .times(1)
        .returning(move |req| {
            *pr_body.lock().unwrap() = req.body;
            Ok(PullRequest {
                number: 1,
                sha: "abc123".into(),
                body: "".into(),
            })
        });
    mock_forge.expect_update_pr().times(0);
    mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

    let orchestrator = create_test_orchestrator(mock_forge);

    orchestrator.create_release_prs(None).await.unwrap();

    let changelog = changelog.lock().unwrap();
    assert!(changelog.contains("https://example.com/commit/c000"));
    assert!(changelog.contains("> The v1 endpoints are gone; migrate to v2."));

    let body = body.lock().unwrap();
    assert!(body.encode_utf16().count() <= LIMIT, "{body}");
    assert!(body.contains("Release notes condensed to fit"), "{body}");
    // group headings come without their `<!-- NN -->` sort tags
    assert!(
        body.contains(
            "### ❌ Breaking\n\
             - [**breaking**]: drop the v1 endpoints _(brk)_\n\
             ### 🐛 Bug Fixes\n- "
        ),
        "{body}"
    );
    assert!(!body.contains("<!-- "), "{body}");
    assert!(
        body.contains(
            "- correct a fairly long commit subject number 0 _(c000)_"
        ),
        "{body}"
    );
    assert!(!body.contains("https://example.com/commit/"), "{body}");
    assert!(!body.contains("The v1 endpoints are gone"), "{body}");
}

#[tokio::test]
async fn create_release_prs_targets_specific_package() {
    let mut mock_forge = MockForge::new();

    // No tags exist yet
    mock_forge
        .expect_get_latest_tags_for_prefix()
        .returning(|_, _, _, _| Ok(vec![]));

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

    mock_forge.expect_get_file_content().returning(|_| Ok(None));

    mock_forge
        .expect_create_release_branch()
        .returning(|_| {
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        })
        .times(1);

    mock_forge.expect_max_pr_body_len().returning(|| None);

    mock_forge
        .expect_encode_pr_metadata()
        .times(1)
        .returning(|json| PrMetadataBlock {
            inline_content: format!("<!--{json}-->"),
            div_attribute: String::new(),
        });

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
        repository: RepositoryConfig {
            separate_pull_requests: true,
            ..RepositoryConfig::default()
        },
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
        .expect_get_latest_tags_for_prefix()
        .returning(|_, _, _, _| {
            Ok(vec![Tag {
                name: "v1.0.0".to_string(),
                version: TagVersion::Semantic(Version::parse("1.0.0").unwrap()),
                sha: "abc123".to_string(),
                timestamp: Some(100),
            }])
        });

    mock_forge.expect_get_commits().returning(|_, _| {
        Ok(vec![
            ForgeCommitBuilder::default()
                .id("def456")
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

    mock_forge.expect_get_file_content().returning(|_| Ok(None));

    mock_forge
        .expect_create_release_branch()
        .returning(|_| {
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        })
        .times(1);

    mock_forge.expect_create_pr().times(0);

    mock_forge.expect_max_pr_body_len().returning(|| None);

    mock_forge
        .expect_encode_pr_metadata()
        .times(1)
        .returning(|json| PrMetadataBlock {
            inline_content: format!("<!--{json}-->"),
            div_attribute: String::new(),
        });

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
