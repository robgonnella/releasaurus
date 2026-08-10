//! Tests for the one-shot release workflow.
//!
//! Tests for:
//! - one_shot method
//! - Commit grouping across a monorepo vs separate_pull_requests
//! - Guarding against a merged-but-untagged release PR
//! - Handling empty releasable packages

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    config::{
        Config, package::PackageConfigBuilder, release_type::ReleaseType,
        repository::RepositoryConfig,
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

/// The release commit has already landed by the time tagging runs, so a
/// failure there is reported with what is on the base branch rather than
/// as a bare forge error.
#[tokio::test]
async fn one_shot_reports_a_partial_release_when_tagging_fails() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    expect_two_package_history(&mut mock_forge);

    mock_forge.expect_get_file_content().returning(|_| Ok(None));

    mock_forge.expect_create_commit().returning(|_| {
        Ok(Commit {
            sha: "release-sha".to_string(),
        })
    });

    let calls = Arc::new(Mutex::new(0));
    let sink = Arc::clone(&calls);

    mock_forge.expect_tag_commit().returning(move |_, _| {
        let mut count = sink.lock().unwrap();
        *count += 1;

        if *count > 1 {
            return Err(ReleasaurusError::forge("tag already exists"));
        }

        Ok(())
    });

    // nothing is published once tagging the group has failed
    mock_forge.expect_create_release().times(0);

    let orchestrator =
        create_test_orchestrator_with_config(mock_forge, two_packages(), None);

    let err = orchestrator.one_shot(None).await.unwrap_err();

    assert!(matches!(
        err,
        ReleasaurusError::PartialOneShotRelease { .. }
    ));
}

/// A publish failure leaves every package in the group tagged, so the
/// state is at least consistent and the error can name all of them.
#[tokio::test]
async fn one_shot_reports_a_partial_release_when_publishing_fails() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    expect_two_package_history(&mut mock_forge);

    mock_forge.expect_get_file_content().returning(|_| Ok(None));

    mock_forge.expect_create_commit().returning(|_| {
        Ok(Commit {
            sha: "release-sha".to_string(),
        })
    });

    let tagged = capture_tagged_commits(&mut mock_forge);

    mock_forge
        .expect_create_release()
        .returning(|_, _, _| Err(ReleasaurusError::forge("409 conflict")));

    let orchestrator =
        create_test_orchestrator_with_config(mock_forge, two_packages(), None);

    let err = orchestrator.one_shot(None).await.unwrap_err();

    assert!(matches!(
        err,
        ReleasaurusError::PartialOneShotRelease { .. }
    ));

    assert_eq!(tagged.lock().unwrap().len(), 2);
}

/// Tagging the whole group before publishing any of it keeps a mid-flight
/// failure from leaving some packages tagged and others not.
#[tokio::test]
async fn one_shot_tags_every_package_before_publishing_any() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    expect_two_package_history(&mut mock_forge);

    mock_forge.expect_get_file_content().returning(|_| Ok(None));

    mock_forge.expect_create_commit().returning(|_| {
        Ok(Commit {
            sha: "release-sha".to_string(),
        })
    });

    let order: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));

    let sink = Arc::clone(&order);
    mock_forge.expect_tag_commit().returning(move |_, _| {
        sink.lock().unwrap().push("tag".to_string());
        Ok(())
    });

    let sink = Arc::clone(&order);
    mock_forge
        .expect_create_release()
        .returning(move |_, _, _| {
            sink.lock().unwrap().push("release".to_string());
            Ok(())
        });

    let orchestrator =
        create_test_orchestrator_with_config(mock_forge, two_packages(), None);

    orchestrator.one_shot(None).await.unwrap();

    let order = order.lock().unwrap();
    assert_eq!(*order, vec!["tag", "tag", "release", "release"]);
}

/// The repo tree as the base branch sees it, mutated by every commit.
type Tree = Arc<Mutex<HashMap<String, String>>>;

/// Backs `get_file_content` and `create_commit` with one mutable tree, so
/// a commit is visible to whatever the next commit reads.
fn expect_committing_tree(
    mock: &mut MockForge,
    files: &[(&str, &str)],
) -> Tree {
    let tree: Tree = Arc::new(Mutex::new(
        files
            .iter()
            .map(|(p, c)| (p.to_string(), c.to_string()))
            .collect(),
    ));

    let reads = Arc::clone(&tree);
    mock.expect_get_file_content().returning(move |req| {
        Ok(reads.lock().unwrap().get(&req.path).cloned())
    });

    let writes = Arc::clone(&tree);
    mock.expect_create_commit().returning(move |req| {
        let mut tree = writes.lock().unwrap();

        for change in req.file_changes.iter() {
            tree.insert(change.repo_path.clone(), change.full_content.clone());
        }

        Ok(Commit {
            sha: "release-sha".to_string(),
        })
    });

    tree
}

/// Two crates in one workspace, each releasing onto its own branch but
/// committing to the same base branch.
fn two_rust_packages() -> Vec<crate::config::package::PackageConfig> {
    ["pkg-a", "pkg-b"]
        .into_iter()
        .map(|name| {
            PackageConfigBuilder::default()
                .name(name)
                .path(format!("packages/{name}"))
                .workspace_root(".")
                .release_type(ReleaseType::Rust)
                .build()
                .unwrap()
        })
        .collect()
}

/// With `separate_pull_requests` each package gets its own commit, but
/// both land on the base branch and both rewrite the workspace root
/// manifest. Building every bundle up front would hand the second commit
/// content read before the first one landed, reverting it.
#[tokio::test]
async fn one_shot_separate_commits_do_not_revert_each_other() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    expect_two_package_history(&mut mock_forge);

    mock_forge.expect_tag_commit().returning(|_, _| Ok(()));
    mock_forge
        .expect_create_release()
        .returning(|_, _, _| Ok(()));

    let tree = expect_committing_tree(
        &mut mock_forge,
        &[
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"packages/pkg-a\", \
                 \"packages/pkg-b\"]\n\n[workspace.dependencies]\npkg-a = \
                 \"0.0.1\"\npkg-b = \"0.0.1\"\n",
            ),
            (
                "packages/pkg-a/Cargo.toml",
                "[package]\nname = \"pkg-a\"\nversion = \"0.0.1\"\n",
            ),
            (
                "packages/pkg-b/Cargo.toml",
                "[package]\nname = \"pkg-b\"\nversion = \"0.0.1\"\n",
            ),
        ],
    );

    let config = Config {
        repository: RepositoryConfig {
            separate_pull_requests: true,
            ..RepositoryConfig::default()
        },
        ..Default::default()
    };

    let orchestrator = create_test_orchestrator_with_config(
        mock_forge,
        two_rust_packages(),
        Some(config),
    );

    orchestrator.one_shot(None).await.unwrap();

    let tree = tree.lock().unwrap();
    let workspace = tree.get("Cargo.toml").unwrap();

    // untagged packages start at 0.1.0
    assert!(
        workspace.contains("pkg-a = \"0.1.0\""),
        "pkg-a's bump was reverted by pkg-b's commit: {workspace}"
    );
    assert!(
        workspace.contains("pkg-b = \"0.1.0\""),
        "pkg-b's bump was reverted by pkg-a's commit: {workspace}"
    );
}
