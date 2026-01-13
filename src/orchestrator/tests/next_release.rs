//! Tests for next release workflow.
//!
//! Tests for:
//! - start_next_release method
//! - Creating commits for tagged packages
//! - Filtering by target packages
//! - Skipping untagged packages

use super::common::*;
use crate::{
    analyzer::release::Tag,
    config::package::PackageConfigBuilder,
    forge::{request::Commit, traits::MockForge},
};

#[tokio::test]
async fn start_next_release_creates_commits_for_tagged_packages() {
    // Set up mock forge expectations FIRST
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_latest_tag_for_prefix()
        .returning(|_| {
            Ok(Some(Tag {
                semver: Version::parse("1.0.0").unwrap(),
                ..Default::default()
            }))
        });

    mock_forge.expect_get_commits().returning(|_, _| Ok(vec![]));

    mock_forge
        .expect_create_commit()
        .times(1)
        .withf(|req| req.message.contains("chore(main): bump patch version"))
        .returning(|_| {
            Ok(Commit {
                sha: "new-sha".to_string(),
            })
        });

    let orchestrator = create_test_orchestrator(mock_forge);

    orchestrator.start_next_release(None).await.unwrap();
}

#[tokio::test]
async fn start_next_release_filters_by_target_packages() {
    // Set up mock forge expectations FIRST
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_latest_tag_for_prefix()
        .times(2)
        .returning(|prefix| {
            if prefix.contains("pkg-a") {
                Ok(Some(Tag {
                    semver: Version::parse("1.0.0").unwrap(),
                    ..Default::default()
                }))
            } else {
                Ok(Some(Tag {
                    semver: Version::parse("2.0.0").unwrap(),
                    ..Default::default()
                }))
            }
        });

    mock_forge.expect_get_commits().returning(|_, _| Ok(vec![]));

    // Should only create commit for pkg-a (the targeted package)
    mock_forge.expect_create_commit().times(1).returning(|_| {
        Ok(Commit {
            sha: "new-sha".to_string(),
        })
    });

    // Create orchestrator with two packages
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

    // Only target pkg-a
    orchestrator
        .start_next_release(Some(vec!["pkg-a".to_string()]))
        .await
        .unwrap();
}

#[tokio::test]
async fn start_next_release_skips_untagged_packages() {
    // Set up mock forge expectations FIRST
    let mut mock_forge = MockForge::new();

    // Return None indicating no tag exists for this package
    mock_forge
        .expect_get_latest_tag_for_prefix()
        .returning(|_| Ok(None));

    // Should NOT call create_commit since package has no tag
    mock_forge.expect_create_commit().times(0);

    let orchestrator = create_test_orchestrator(mock_forge);

    // Should complete without error, just skip the untagged package
    orchestrator.start_next_release(None).await.unwrap();
}
