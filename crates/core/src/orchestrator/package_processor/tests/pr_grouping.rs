//! Tests for PR grouping and branch logic.
//!
//! Tests for:
//! - Grouping all packages when not configured for separate PRs
//! - Separating packages into different branches when configured

use semver::Version;

use crate::{
    config::{
        Config, package::PackageConfigBuilder, repository::RepositoryConfig,
    },
    forge::{request::Tag, traits::MockForge},
    packages::releasable::ReleasablePackage,
};

use super::common::*;

#[tokio::test]
async fn release_pr_bundles_groups_all_when_not_separate() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_open_release_pr()
        .returning(|_| Ok(None));

    let toml_config = Config {
        repository: RepositoryConfig {
            separate_pull_requests: false,
            ..RepositoryConfig::default()
        },
        ..Config::default()
    };

    let pkg_a_config = PackageConfigBuilder::default()
        .name("pkg-a")
        .path("packages/pkg-a")
        .build()
        .unwrap();

    let pkg_b_config = PackageConfigBuilder::default()
        .name("pkg-b")
        .path("packages/pkg-b")
        .build()
        .unwrap();

    let processor = create_package_processor(
        mock_forge,
        Some(vec![pkg_a_config, pkg_b_config]),
        Some(toml_config),
    );

    let releasable_a = ReleasablePackage {
        name: "pkg-a".to_string(),
        tag: Tag {
            name: "v1.0.0".to_string(),
            semver: Version::parse("1.0.0").unwrap(),
            ..Default::default()
        },
        notes: "Release A".to_string(),
        ..Default::default()
    };

    let releasable_b = ReleasablePackage {
        name: "pkg-b".to_string(),
        tag: Tag {
            name: "v2.0.0".to_string(),
            semver: Version::parse("2.0.0").unwrap(),
            ..Default::default()
        },
        notes: "Release B".to_string(),
        ..Default::default()
    };

    let groups =
        processor.group_releasable_packages(vec![releasable_a, releasable_b]);

    let grouped = processor.release_pr_bundles(groups).await.unwrap();
    // Should have one branch with both packages
    assert_eq!(grouped.len(), 1);

    let bundle = grouped.first().unwrap();
    assert_eq!(bundle.packages.len(), 2);
}

#[tokio::test]
async fn release_pr_bundles_separates_when_configured() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_open_release_pr()
        .returning(|_| Ok(None));

    let toml_config = Config {
        repository: RepositoryConfig {
            separate_pull_requests: true,
            ..RepositoryConfig::default()
        },
        ..Config::default()
    };

    let pkg_a_config = PackageConfigBuilder::default()
        .name("pkg-a")
        .path("packages/pkg-a")
        .build()
        .unwrap();

    let pkg_b_config = PackageConfigBuilder::default()
        .name("pkg-b")
        .path("packages/pkg-b")
        .build()
        .unwrap();

    let processor = create_package_processor(
        mock_forge,
        Some(vec![pkg_a_config, pkg_b_config]),
        Some(toml_config),
    );

    let releasable_a = ReleasablePackage {
        name: "pkg-a".to_string(),
        tag: Tag {
            name: "v1.0.0".to_string(),
            semver: Version::parse("1.0.0").unwrap(),
            ..Default::default()
        },
        notes: "Release A".to_string(),
        ..Default::default()
    };

    let releasable_b = ReleasablePackage {
        name: "pkg-b".to_string(),
        tag: Tag {
            name: "v2.0.0".to_string(),
            semver: Version::parse("2.0.0").unwrap(),
            ..Default::default()
        },
        notes: "Release B".to_string(),
        ..Default::default()
    };

    let groups =
        processor.group_releasable_packages(vec![releasable_a, releasable_b]);

    let grouped = processor.release_pr_bundles(groups).await.unwrap();
    // Should have separate branches
    assert_eq!(grouped.len(), 2);

    for bundle in grouped.iter() {
        assert_eq!(bundle.packages.len(), 1);
    }
}
