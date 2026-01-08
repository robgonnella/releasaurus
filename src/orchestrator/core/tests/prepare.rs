//! Tests for package preparation functionality.
//!
//! Tests for:
//! - Generating prepared packages with dummy commits
//! - Skipping untagged packages
//! - Filtering by target packages

use super::common::*;
use crate::{
    analyzer::release::Tag, config::package::PackageConfigBuilder,
    forge::traits::MockForge,
};

#[tokio::test]
async fn generate_prepared_with_dummy_commit_skips_untagged_packages() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_latest_tag_for_prefix()
        .returning(|_| Ok(None)); // No tags exist

    let orchestrator = create_core(mock_forge, None, None);

    let prepared = orchestrator
        .generate_prepared_with_dummy_commit(None)
        .await
        .unwrap();
    // Should skip untagged package
    assert_eq!(prepared.len(), 0);
}

#[tokio::test]
async fn generate_prepared_with_dummy_commit_filters_by_targets() {
    let pkg_configs = vec![
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
    ];

    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_latest_tag_for_prefix()
        .returning(|prefix| {
            Ok(Some(Tag {
                name: format!("{prefix}1.0.0"),
                timestamp: Some(1000),
                ..Default::default()
            }))
        });

    let orchestrator = create_core(mock_forge, Some(pkg_configs), None);

    let prepared = orchestrator
        .generate_prepared_with_dummy_commit(Some(vec!["pkg-a".to_string()]))
        .await
        .unwrap();
    // Should only include pkg-a
    assert_eq!(prepared.len(), 1);
    assert_eq!(prepared[0].name, "pkg-a");
}
