//! Tests for PR grouping and branch logic.
//!
//! Tests for:
//! - Grouping all packages when not configured for separate PRs
//! - Separating packages into different branches when configured

use super::common::*;
use crate::{
    analyzer::release::Tag,
    config::{Config, package::PackageConfigBuilder},
    forge::traits::MockForge,
};

#[test]
fn release_pr_packages_by_branch_groups_all_when_not_separate() {
    let mock_forge = MockForge::new();

    let toml_config = Config {
        separate_pull_requests: false,
        ..Config::default()
    };

    let pkg_a_config = PackageConfigBuilder::default()
        .name("pkg-a")
        .path(".")
        .build()
        .unwrap();

    let pkg_b_config = PackageConfigBuilder::default()
        .name("pkg-b")
        .path(".")
        .build()
        .unwrap();

    let orchestrator = create_core(
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

    let grouped = orchestrator
        .release_pr_packages_by_branch(vec![releasable_a, releasable_b])
        .unwrap();
    // Should have one branch with both packages
    assert_eq!(grouped.len(), 1);

    let packages = grouped.values().next().unwrap();
    assert_eq!(packages.len(), 2);
}

#[test]
fn release_pr_packages_by_branch_separates_when_configured() {
    let mock_forge = MockForge::new();

    let toml_config = Config {
        separate_pull_requests: true,
        ..Config::default()
    };

    let pkg_a_config = PackageConfigBuilder::default()
        .name("pkg-a")
        .path(".")
        .build()
        .unwrap();

    let pkg_b_config = PackageConfigBuilder::default()
        .name("pkg-b")
        .path(".")
        .build()
        .unwrap();

    let orchestrator = create_core(
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

    let grouped = orchestrator
        .release_pr_packages_by_branch(vec![releasable_a, releasable_b])
        .unwrap();
    // Should have separate branches
    assert_eq!(grouped.len(), 2);

    for packages in grouped.values() {
        assert_eq!(packages.len(), 1);
    }
}
