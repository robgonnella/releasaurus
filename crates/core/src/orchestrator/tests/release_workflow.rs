//! Tests for release workflow.
//!
//! Tests for:
//! - create_releases method
//! - Finding merged PRs and creating releases
//! - Auto-start next release behavior
//! - Handling separate pull requests
//! - Skipping packages without merged PRs
//! - Edited notes and header/footer sections in release notes

use crate::{
    config::{
        Config, package::PackageConfigBuilder, repository::RepositoryConfig,
        versioning::VersioningConfig,
    },
    forge::{
        request::{Commit, GetPrRequest, PullRequest, Tag, TagResponse},
        traits::MockForge,
    },
    result::ReleasaurusError,
};

use super::common::*;

#[tokio::test]
async fn create_releases_skips_packages_without_merged_pr() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .times(1)
        .returning(|_| Ok(None));

    // Should NOT call tag_commit, create_release, or replace_pr_labels
    mock_forge.expect_tag_commit().times(0);
    mock_forge.expect_create_release().times(0);
    mock_forge.expect_replace_pr_labels().times(0);

    let orchestrator = create_test_orchestrator(mock_forge);

    orchestrator.create_releases(None).await.unwrap();
}

#[tokio::test]
async fn create_releases_returns_error_for_invalid_package_name() {
    let mock_forge = MockForge::new();

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
        None,
    );

    let err = orchestrator
        .create_releases(Some("nope".into()))
        .await
        .unwrap_err();

    assert!(matches!(err, ReleasaurusError::InvalidArgs(_)));
}

#[tokio::test]
async fn create_releases_handles_separate_pull_requests() {
    let pr_body_a = make_pr_body(&PrBodyInput {
        pkg: "pkg-a",
        tag: "v1.0.0",
        notes: "Release A",
        tag_link: "tag-link-a",
        sha_link: "sha-link-a",
        header: "",
        footer: "",
    });
    let pr_body_b = make_pr_body(&PrBodyInput {
        pkg: "pkg-b",
        tag: "v2.0.0",
        notes: "Release B",
        tag_link: "tag-link-b",
        sha_link: "sha-link-b",
        header: "",
        footer: "",
    });

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

    mock_forge.expect_get_tag().returning(|_| Ok(None));

    mock_forge
        .expect_tag_commit()
        .times(2)
        .returning(|_, _| Ok(()));
    mock_forge
        .expect_create_release()
        .times(2)
        .returning(|_| Ok(()));
    mock_forge
        .expect_replace_pr_labels()
        .times(2)
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

    orchestrator.create_releases(None).await.unwrap();
}

#[tokio::test]
async fn create_releases_targets_specific_package() {
    let pr_body_a = make_pr_body(&PrBodyInput {
        pkg: "pkg-a",
        tag: "pkg-a-v1.0.0",
        notes: "Release A",
        tag_link: "tag-link-a",
        sha_link: "sha-link-a",
        header: "",
        footer: "",
    });

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

    mock_forge.expect_get_tag().returning(|_| Ok(None));

    mock_forge
        .expect_tag_commit()
        .withf(|tag, _| tag.contains("pkg-a"))
        .times(1)
        .returning(|_, _| Ok(()));
    mock_forge
        .expect_create_release()
        .withf(|req| req.tag.contains("pkg-a"))
        .times(1)
        .returning(|_| Ok(()));
    mock_forge
        .expect_replace_pr_labels()
        .times(1)
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

#[tokio::test]
async fn create_releases_triggers_auto_start_next() {
    let pr_body = make_pr_body(&PrBodyInput {
        pkg: TEST_PKG_NAME,
        tag: "v1.0.0",
        notes: "Release notes",
        tag_link: "tag-link",
        sha_link: "sha-link",
        header: "",
        footer: "",
    });

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

    mock_forge.expect_get_tag().returning(|_| Ok(None));
    mock_forge.expect_tag_commit().returning(|_, _| Ok(()));
    mock_forge.expect_create_release().returning(|_| Ok(()));
    mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

    mock_forge
        .expect_get_latest_tags_for_prefix()
        .returning(|_, _, _| {
            Ok(vec![Tag {
                semver: Version::parse("1.0.0").unwrap(),
                ..Default::default()
            }])
        });

    mock_forge.expect_get_commits().returning(|_, _| Ok(vec![]));

    mock_forge.expect_get_file_content().returning(|_| Ok(None));

    mock_forge.expect_create_commit().times(1).returning(|_| {
        Ok(Commit {
            sha: "new-commit".to_string(),
        })
    });

    let versioning = VersioningConfig {
        auto_start_next: Some(true),
        ..Default::default()
    };

    let orchestrator = create_test_orchestrator_with_config(
        mock_forge,
        vec![
            PackageConfigBuilder::default()
                .name(TEST_PKG_NAME)
                .path(".")
                .versioning(versioning)
                .build()
                .unwrap(),
        ],
        None,
    );

    orchestrator.create_releases(None).await.unwrap();
}

#[tokio::test]
async fn create_releases_uses_edited_notes_from_pr_body() {
    let edited_notes = "User-edited release notes for this version";
    let pr_body = make_pr_body(&PrBodyInput {
        pkg: TEST_PKG_NAME,
        tag: "v1.0.0",
        notes: edited_notes,
        tag_link: "tag-link",
        sha_link: "sha-link",
        header: "",
        footer: "",
    });

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

    mock_forge.expect_get_tag().returning(|_| Ok(None));

    mock_forge
        .expect_tag_commit()
        .times(1)
        .withf(|tag, sha| tag == "v1.0.0" && sha == "abc123")
        .returning(|_, _| Ok(()));

    mock_forge
        .expect_create_release()
        .times(1)
        .withf(|req| {
            req.tag == "v1.0.0"
                && req.sha == "abc123"
                && req.notes.contains("User-edited release notes")
        })
        .returning(|_| Ok(()));

    mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

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
async fn create_releases_includes_header_and_footer_in_release_notes() {
    let pr_body = make_pr_body(&PrBodyInput {
        pkg: TEST_PKG_NAME,
        tag: "v1.0.0",
        notes: "Release notes",
        tag_link: "tag-link",
        sha_link: "sha-link",
        header: "Custom header text",
        footer: "Custom footer text",
    });

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

    mock_forge.expect_get_tag().returning(|_| Ok(None));
    mock_forge.expect_tag_commit().returning(|_, _| Ok(()));

    mock_forge
        .expect_create_release()
        .times(1)
        .withf(|req| {
            req.notes.contains("Custom header text")
                && req.notes.contains("Release notes")
                && req.notes.contains("Custom footer text")
        })
        .returning(|_| Ok(()));

    mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

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

/// A run that tagged but failed before publishing leaves the PR pending,
/// so the next run comes back through here. Re-tagging would conflict —
/// the existing tag is recognised and only the release is created.
#[tokio::test]
async fn create_releases_skips_tagging_when_the_tag_already_exists() {
    let pr_body = make_pr_body(&PrBodyInput {
        pkg: TEST_PKG_NAME,
        tag: "v1.0.0",
        notes: "Release notes",
        tag_link: "tag-link",
        sha_link: "sha-link",
        header: "",
        footer: "",
    });

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(move |_| {
            Ok(Some(PullRequest {
                number: 123,
                sha: "abc123".to_string(),
                body: pr_body.clone(),
            }))
        });

    mock_forge.expect_get_tag().returning(|tag| {
        Ok(Some(TagResponse {
            tag: tag.to_string(),
            sha: "abc123".to_string(),
        }))
    });

    mock_forge.expect_tag_commit().times(0);

    mock_forge
        .expect_create_release()
        .times(1)
        .withf(|req| req.tag == "v1.0.0" && req.sha == "abc123")
        .returning(|_| Ok(()));

    mock_forge
        .expect_replace_pr_labels()
        .times(1)
        .returning(|_| Ok(()));

    let orchestrator = create_test_orchestrator(mock_forge);

    orchestrator.create_releases(None).await.unwrap();
}

/// A tag of the same name pointing at a different commit is not the one
/// we created, so tagging must still be attempted rather than assumed
/// done.
#[tokio::test]
async fn create_releases_still_tags_when_an_existing_tag_points_elsewhere() {
    let pr_body = make_pr_body(&PrBodyInput {
        pkg: TEST_PKG_NAME,
        tag: "v1.0.0",
        notes: "Release notes",
        tag_link: "tag-link",
        sha_link: "sha-link",
        header: "",
        footer: "",
    });

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(move |_| {
            Ok(Some(PullRequest {
                number: 123,
                sha: "abc123".to_string(),
                body: pr_body.clone(),
            }))
        });

    mock_forge.expect_get_tag().returning(|tag| {
        Ok(Some(TagResponse {
            tag: tag.to_string(),
            sha: "a-different-sha".to_string(),
        }))
    });

    mock_forge
        .expect_tag_commit()
        .times(1)
        .returning(|_, _| Ok(()));

    mock_forge
        .expect_create_release()
        .times(1)
        .returning(|_| Ok(()));

    mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

    let orchestrator = create_test_orchestrator(mock_forge);

    orchestrator.create_releases(None).await.unwrap();
}

// Prerelease flag on the published release. The merged PR body carries
// only the tag name, so the flag is recovered by stripping the package's
// tag prefix and parsing the remainder as semver.

#[tokio::test]
async fn create_releases_marks_a_prerelease_tag_as_a_prerelease() {
    let pr_body = make_pr_body(&PrBodyInput {
        pkg: TEST_PKG_NAME,
        tag: "v1.0.0-rc.1",
        notes: "Release notes",
        tag_link: "tag-link",
        sha_link: "sha-link",
        header: "",
        footer: "",
    });

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(move |_| {
            Ok(Some(PullRequest {
                number: 123,
                sha: "abc123".to_string(),
                body: pr_body.clone(),
            }))
        });

    mock_forge.expect_get_tag().returning(|tag| {
        Ok(Some(TagResponse {
            tag: tag.to_string(),
            sha: "abc123".to_string(),
        }))
    });

    mock_forge
        .expect_create_release()
        .times(1)
        .withf(|req| req.tag == "v1.0.0-rc.1" && req.prerelease)
        .returning(|_| Ok(()));

    mock_forge
        .expect_replace_pr_labels()
        .times(1)
        .returning(|_| Ok(()));

    let orchestrator = create_test_orchestrator(mock_forge);

    orchestrator.create_releases(None).await.unwrap();
}

#[tokio::test]
async fn create_releases_does_not_mark_a_stable_tag_as_a_prerelease() {
    let pr_body = make_pr_body(&PrBodyInput {
        pkg: TEST_PKG_NAME,
        tag: "v1.0.0",
        notes: "Release notes",
        tag_link: "tag-link",
        sha_link: "sha-link",
        header: "",
        footer: "",
    });

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(move |_| {
            Ok(Some(PullRequest {
                number: 123,
                sha: "abc123".to_string(),
                body: pr_body.clone(),
            }))
        });

    mock_forge.expect_get_tag().returning(|tag| {
        Ok(Some(TagResponse {
            tag: tag.to_string(),
            sha: "abc123".to_string(),
        }))
    });

    mock_forge
        .expect_create_release()
        .times(1)
        .withf(|req| !req.prerelease)
        .returning(|_| Ok(()));

    mock_forge
        .expect_replace_pr_labels()
        .times(1)
        .returning(|_| Ok(()));

    let orchestrator = create_test_orchestrator(mock_forge);

    orchestrator.create_releases(None).await.unwrap();
}

/// The prefix itself contains a hyphen, so prefix stripping must not
/// break prerelease detection. Paired with the stable case below, which
/// is what actually rules out looking for a hyphen in the whole tag.
#[tokio::test]
async fn create_releases_detects_a_prerelease_behind_a_hyphenated_tag_prefix() {
    let pr_body = make_pr_body(&PrBodyInput {
        pkg: "mypkg",
        tag: "mypkg-v1.0.0-rc.1",
        notes: "Release notes",
        tag_link: "tag-link",
        sha_link: "sha-link",
        header: "",
        footer: "",
    });

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(move |_| {
            Ok(Some(PullRequest {
                number: 123,
                sha: "abc123".to_string(),
                body: pr_body.clone(),
            }))
        });

    mock_forge.expect_get_tag().returning(|tag| {
        Ok(Some(TagResponse {
            tag: tag.to_string(),
            sha: "abc123".to_string(),
        }))
    });

    mock_forge
        .expect_create_release()
        .times(1)
        .withf(|req| req.tag == "mypkg-v1.0.0-rc.1" && req.prerelease)
        .returning(|_| Ok(()));

    mock_forge
        .expect_replace_pr_labels()
        .times(1)
        .returning(|_| Ok(()));

    let orchestrator = create_test_orchestrator_with_config(
        mock_forge,
        vec![
            PackageConfigBuilder::default()
                .name("mypkg")
                .path(".")
                .tag_prefix("mypkg-v")
                .build()
                .unwrap(),
        ],
        None,
    );

    orchestrator.create_releases(None).await.unwrap();
}

/// Build metadata lands in semver's `build`, not `pre`, so a version
/// carrying it is still a normal release.
#[tokio::test]
async fn create_releases_treats_build_metadata_as_a_stable_release() {
    let pr_body = make_pr_body(&PrBodyInput {
        pkg: TEST_PKG_NAME,
        tag: "v1.0.0+20260824.abc123",
        notes: "Release notes",
        tag_link: "tag-link",
        sha_link: "sha-link",
        header: "",
        footer: "",
    });

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(move |_| {
            Ok(Some(PullRequest {
                number: 123,
                sha: "abc123".to_string(),
                body: pr_body.clone(),
            }))
        });

    mock_forge.expect_get_tag().returning(|tag| {
        Ok(Some(TagResponse {
            tag: tag.to_string(),
            sha: "abc123".to_string(),
        }))
    });

    mock_forge
        .expect_create_release()
        .times(1)
        .withf(|req| !req.prerelease)
        .returning(|_| Ok(()));

    mock_forge
        .expect_replace_pr_labels()
        .times(1)
        .returning(|_| Ok(()));

    let orchestrator = create_test_orchestrator(mock_forge);

    orchestrator.create_releases(None).await.unwrap();
}

/// A hand-edited `data-tag` that doesn't parse must not block the
/// release — it falls back to publishing a normal release.
#[tokio::test]
async fn create_releases_falls_back_to_stable_for_an_unparseable_tag() {
    let pr_body = make_pr_body(&PrBodyInput {
        pkg: TEST_PKG_NAME,
        tag: "v-not-a-version",
        notes: "Release notes",
        tag_link: "tag-link",
        sha_link: "sha-link",
        header: "",
        footer: "",
    });

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(move |_| {
            Ok(Some(PullRequest {
                number: 123,
                sha: "abc123".to_string(),
                body: pr_body.clone(),
            }))
        });

    mock_forge.expect_get_tag().returning(|tag| {
        Ok(Some(TagResponse {
            tag: tag.to_string(),
            sha: "abc123".to_string(),
        }))
    });

    mock_forge
        .expect_create_release()
        .times(1)
        .withf(|req| !req.prerelease)
        .returning(|_| Ok(()));

    mock_forge
        .expect_replace_pr_labels()
        .times(1)
        .returning(|_| Ok(()));

    let orchestrator = create_test_orchestrator(mock_forge);

    orchestrator.create_releases(None).await.unwrap();
}

/// The discriminating case for the hyphenated prefix: a hyphen appears in
/// `mypkg-v1.0.0`, but only in the prefix, so this is a stable release.
#[tokio::test]
async fn create_releases_does_not_mark_a_stable_tag_behind_a_hyphenated_prefix()
{
    let pr_body = make_pr_body(&PrBodyInput {
        pkg: "mypkg",
        tag: "mypkg-v1.0.0",
        notes: "Release notes",
        tag_link: "tag-link",
        sha_link: "sha-link",
        header: "",
        footer: "",
    });

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(move |_| {
            Ok(Some(PullRequest {
                number: 123,
                sha: "abc123".to_string(),
                body: pr_body.clone(),
            }))
        });

    mock_forge.expect_get_tag().returning(|tag| {
        Ok(Some(TagResponse {
            tag: tag.to_string(),
            sha: "abc123".to_string(),
        }))
    });

    mock_forge
        .expect_create_release()
        .times(1)
        .withf(|req| req.tag == "mypkg-v1.0.0" && !req.prerelease)
        .returning(|_| Ok(()));

    mock_forge
        .expect_replace_pr_labels()
        .times(1)
        .returning(|_| Ok(()));

    let orchestrator = create_test_orchestrator_with_config(
        mock_forge,
        vec![
            PackageConfigBuilder::default()
                .name("mypkg")
                .path(".")
                .tag_prefix("mypkg-v")
                .build()
                .unwrap(),
        ],
        None,
    );

    orchestrator.create_releases(None).await.unwrap();
}
