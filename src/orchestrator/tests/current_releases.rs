//! Tests for release data retrieval.
//!
//! Tests for:
//! - get_current_releases method
//! - get_next_releases method
//! - Filtering by package name

use super::common::*;
use crate::{
    analyzer::release::Tag,
    config::package::PackageConfigBuilder,
    forge::{
        request::ForgeCommitBuilder, request::ReleaseByTagResponse,
        traits::MockForge,
    },
};

#[tokio::test]
async fn get_current_releases_retrieves_release_data() {
    // Set up mock forge expectations FIRST
    let mut mock_forge = MockForge::new();
    mock_forge.expect_dry_run().returning(|| false);

    mock_forge
        .expect_get_latest_tag_for_prefix()
        .returning(|_| {
            Ok(Some(Tag {
                name: "v1.0.0".to_string(),
                ..Default::default()
            }))
        });

    mock_forge
        .expect_get_release_by_tag()
        .times(1)
        .withf(|tag| tag == "v1.0.0")
        .returning(|_| {
            Ok(ReleaseByTagResponse {
                tag: "v1.0.0".to_string(),
                sha: "abc123".to_string(),
                notes: "Release notes".to_string(),
            })
        });

    let orchestrator = create_test_orchestrator(mock_forge);

    let releases = orchestrator.get_current_releases(None).await.unwrap();
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].tag, "v1.0.0");
}

#[tokio::test]
async fn get_next_releases_filters_by_package_name() {
    // Set up mock forge expectations FIRST
    let mut mock_forge = MockForge::new();
    mock_forge.expect_dry_run().returning(|| false);

    mock_forge
        .expect_get_latest_tag_for_prefix()
        .returning(|_| Ok(None));

    mock_forge.expect_get_commits().returning(|_, _| {
        Ok(vec![
            ForgeCommitBuilder::default()
                .id("commit1")
                .short_id("c1")
                .message("feat: new feature")
                .timestamp(1000)
                .files(vec!["src/main.rs".to_string()])
                .build()
                .unwrap(),
        ])
    });

    // Create orchestrator with two packages
    let orchestrator = create_test_orchestrator_with_config(
        mock_forge,
        vec![
            PackageConfigBuilder::default()
                .name("pkg-a")
                .path(".")
                .build()
                .unwrap(),
            PackageConfigBuilder::default()
                .name("pkg-b")
                .path(".")
                .build()
                .unwrap(),
        ],
        None,
    );

    // Filter by pkg-a only
    let releases = orchestrator
        .get_next_releases(Some("pkg-a".to_string()))
        .await
        .unwrap();
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].name, "pkg-a");
}
