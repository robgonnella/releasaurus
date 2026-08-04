//! Tests for the one-shot release workflow.
//!
//! Tests for:
//! - one_shot method
//! - Commit grouping across a monorepo vs separate_pull_requests
//! - Guarding against a merged-but-untagged release PR
//! - Handling empty releasable packages

use std::sync::{Arc, Mutex};

use crate::{
    config::{
        Config, package::PackageConfigBuilder, repository::RepositoryConfig,
    },
    forge::{
        request::{Commit, ForgeCommitBuilder, PullRequest, Tag},
        traits::MockForge,
    },
    result::ReleasaurusError,
};

use super::common::*;

/// Records the `(tag, sha)` pairs passed to `tag_commit`.
fn capture_tagged_commits(
    mock: &mut MockForge,
) -> Arc<Mutex<Vec<(String, String)>>> {
    let captured: Arc<Mutex<Vec<(String, String)>>> =
        Arc::new(Mutex::new(vec![]));
    let sink = Arc::clone(&captured);

    mock.expect_tag_commit().returning(move |tag, sha| {
        sink.lock()
            .unwrap()
            .push((tag.to_string(), sha.to_string()));
        Ok(())
    });

    captured
}

/// Two packages, each with their own commit, no tags yet.
fn expect_two_package_history(mock: &mut MockForge) {
    mock.expect_get_latest_tags_for_prefix()
        .returning(|_, _, _| Ok(vec![]));

    mock.expect_get_commits().returning(|_, _| {
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
}

fn two_packages() -> Vec<crate::config::package::PackageConfig> {
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
    ]
}

#[tokio::test]
async fn one_shot_commits_tags_and_releases_a_single_package() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .times(1)
        .returning(|_| Ok(None));

    mock_forge
        .expect_get_latest_tags_for_prefix()
        .returning(|_, _, _| Ok(vec![]));

    mock_forge.expect_get_commits().returning(|_, _| {
        Ok(vec![
            ForgeCommitBuilder::default()
                .id("abc123")
                .files(vec!["dummy.txt".into()])
                .build()
                .unwrap(),
        ])
    });

    mock_forge.expect_get_file_content().returning(|_| Ok(None));

    mock_forge.expect_create_commit().times(1).returning(|_| {
        Ok(Commit {
            sha: "release-sha".to_string(),
        })
    });

    let tagged = capture_tagged_commits(&mut mock_forge);

    mock_forge
        .expect_create_release()
        .times(1)
        .returning(|_, _, _| Ok(()));

    // no PR machinery should be touched by this flow
    mock_forge.expect_get_open_release_pr().times(0);
    mock_forge.expect_create_release_branch().times(0);
    mock_forge.expect_create_pr().times(0);
    mock_forge.expect_replace_pr_labels().times(0);

    let orchestrator = create_test_orchestrator(mock_forge);

    orchestrator.one_shot(None).await.unwrap();

    let tagged = tagged.lock().unwrap();
    assert_eq!(tagged.len(), 1);
    assert_eq!(tagged[0].1, "release-sha");
}

#[tokio::test]
async fn one_shot_creates_one_commit_for_a_monorepo_release() {
    let mut mock_forge = MockForge::new();

    // all packages share a release branch, so the guard looks it up once
    mock_forge
        .expect_get_merged_release_pr()
        .times(1)
        .returning(|_| Ok(None));

    expect_two_package_history(&mut mock_forge);

    // one commit carrying both packages' file changes
    mock_forge.expect_get_file_content().returning(|_| Ok(None));

    mock_forge.expect_create_commit().times(1).returning(|_| {
        Ok(Commit {
            sha: "release-sha".to_string(),
        })
    });

    let tagged = capture_tagged_commits(&mut mock_forge);

    mock_forge
        .expect_create_release()
        .times(2)
        .returning(|_, _, _| Ok(()));

    let orchestrator =
        create_test_orchestrator_with_config(mock_forge, two_packages(), None);

    orchestrator.one_shot(None).await.unwrap();

    let tagged = tagged.lock().unwrap();
    assert_eq!(tagged.len(), 2);

    for (_, sha) in tagged.iter() {
        assert_eq!(sha, "release-sha");
    }

    let mut names: Vec<&str> =
        tagged.iter().map(|(tag, _)| tag.as_str()).collect();
    names.sort_unstable();
    assert!(names[0].contains("pkg-a"), "unexpected tags: {names:?}");
    assert!(names[1].contains("pkg-b"), "unexpected tags: {names:?}");
}

#[tokio::test]
async fn one_shot_creates_a_commit_per_package_when_prs_are_separate() {
    let mut mock_forge = MockForge::new();

    // separate branches per package means one lookup each
    mock_forge
        .expect_get_merged_release_pr()
        .times(2)
        .returning(|_| Ok(None));

    expect_two_package_history(&mut mock_forge);

    mock_forge.expect_get_file_content().returning(|_| Ok(None));

    mock_forge.expect_create_commit().times(2).returning(|_| {
        Ok(Commit {
            sha: "release-sha".to_string(),
        })
    });

    mock_forge
        .expect_tag_commit()
        .times(2)
        .returning(|_, _| Ok(()));

    mock_forge
        .expect_create_release()
        .times(2)
        .returning(|_, _, _| Ok(()));

    let config = Config {
        repository: RepositoryConfig {
            separate_pull_requests: true,
            ..RepositoryConfig::default()
        },
        ..Default::default()
    };

    let orchestrator = create_test_orchestrator_with_config(
        mock_forge,
        two_packages(),
        Some(config),
    );

    orchestrator.one_shot(None).await.unwrap();
}

#[tokio::test]
async fn one_shot_targets_a_specific_package() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .times(1)
        .returning(|_| Ok(None));

    expect_two_package_history(&mut mock_forge);

    mock_forge.expect_get_file_content().returning(|_| Ok(None));

    mock_forge.expect_create_commit().times(1).returning(|_| {
        Ok(Commit {
            sha: "release-sha".to_string(),
        })
    });

    mock_forge
        .expect_tag_commit()
        .withf(|tag, _| tag.contains("pkg-a"))
        .times(1)
        .returning(|_, _| Ok(()));

    mock_forge
        .expect_create_release()
        .withf(|tag, _, _| tag.contains("pkg-a"))
        .times(1)
        .returning(|_, _, _| Ok(()));

    let orchestrator =
        create_test_orchestrator_with_config(mock_forge, two_packages(), None);

    orchestrator.one_shot(Some("pkg-a".into())).await.unwrap();
}

#[tokio::test]
async fn one_shot_does_nothing_when_no_commits_since_last_tag() {
    let mut mock_forge = MockForge::new();

    // nothing releasable means no branch worth checking
    mock_forge.expect_get_merged_release_pr().times(0);

    mock_forge
        .expect_get_latest_tags_for_prefix()
        .returning(|_, _, _| {
            Ok(vec![Tag {
                name: "v1.0.0".to_string(),
                semver: Version::parse("1.0.0").unwrap(),
                sha: "abc123".to_string(),
                timestamp: Some(1234567890),
            }])
        });

    mock_forge.expect_get_commits().returning(|_, _| Ok(vec![]));

    mock_forge.expect_create_commit().times(0);
    mock_forge.expect_tag_commit().times(0);
    mock_forge.expect_create_release().times(0);

    let orchestrator = create_test_orchestrator(mock_forge);

    orchestrator.one_shot(None).await.unwrap();
}

#[tokio::test]
async fn one_shot_returns_error_when_merged_pr_not_yet_released() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_latest_tags_for_prefix()
        .returning(|_, _, _| Ok(vec![]));

    mock_forge.expect_get_commits().returning(|_, _| {
        Ok(vec![
            ForgeCommitBuilder::default()
                .id("abc123")
                .files(vec!["dummy.txt".into()])
                .build()
                .unwrap(),
        ])
    });

    mock_forge
        .expect_get_merged_release_pr()
        .times(1)
        .returning(|_| {
            Ok(Some(PullRequest {
                number: 42,
                sha: "def456".into(),
                body: "".into(),
            }))
        });

    // the guard runs before anything is written
    mock_forge.expect_create_commit().times(0);
    mock_forge.expect_tag_commit().times(0);
    mock_forge.expect_create_release().times(0);

    let orchestrator = create_test_orchestrator(mock_forge);

    let err = orchestrator.one_shot(None).await.unwrap_err();

    assert!(matches!(err, ReleasaurusError::PendingRelease { .. }));
}

/// A package with a pending release but nothing new of its own must not
/// block a package that does have commits to release.
#[tokio::test]
async fn one_shot_ignores_a_pending_release_on_an_unrelated_package() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_latest_tags_for_prefix()
        .returning(|_, _, _| Ok(vec![]));

    mock_forge.expect_get_commits().returning(|_, _| {
        Ok(vec![
            ForgeCommitBuilder::default()
                .id("abc123")
                .files(vec!["packages/pkg-a/dummy.txt".into()])
                .build()
                .unwrap(),
        ])
    });

    // pkg-b sits on a merged, untagged release PR; pkg-a's branch is clear
    mock_forge
        .expect_get_merged_release_pr()
        .times(1)
        .returning(|req| {
            if req.head_branch.ends_with("pkg-b") {
                return Ok(Some(PullRequest {
                    number: 42,
                    sha: "def456".into(),
                    body: "".into(),
                }));
            }
            Ok(None)
        });

    mock_forge.expect_get_file_content().returning(|_| Ok(None));

    mock_forge.expect_create_commit().times(1).returning(|_| {
        Ok(Commit {
            sha: "release-sha".to_string(),
        })
    });

    mock_forge
        .expect_tag_commit()
        .withf(|tag, _| tag.contains("pkg-a"))
        .times(1)
        .returning(|_, _| Ok(()));

    mock_forge
        .expect_create_release()
        .times(1)
        .returning(|_, _, _| Ok(()));

    let config = Config {
        repository: RepositoryConfig {
            separate_pull_requests: true,
            ..RepositoryConfig::default()
        },
        ..Default::default()
    };

    let orchestrator = create_test_orchestrator_with_config(
        mock_forge,
        two_packages(),
        Some(config),
    );

    orchestrator.one_shot(None).await.unwrap();
}

#[tokio::test]
async fn one_shot_returns_error_for_invalid_package_name() {
    let mut mock_forge = MockForge::new();

    // target validation happens before the forge is consulted
    mock_forge.expect_get_merged_release_pr().times(0);

    let orchestrator =
        create_test_orchestrator_with_config(mock_forge, two_packages(), None);

    let err = orchestrator
        .one_shot(Some("nope".into()))
        .await
        .unwrap_err();

    assert!(matches!(err, ReleasaurusError::InvalidArgs(_)));
}
