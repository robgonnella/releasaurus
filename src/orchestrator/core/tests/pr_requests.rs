//! Tests for PR request generation and branch creation.
//!
//! Tests for:
//! - Creating release branches before generating PR requests
//! - PR request creation with proper metadata
//! - PR body formatting with metadata
//! - Single vs multiple package PR handling

use super::common::*;
use crate::{
    analyzer::release::Tag,
    config::{Config, package::PackageConfigBuilder},
    forge::{
        request::{Commit, CreateReleaseBranchRequest},
        traits::MockForge,
    },
};

#[tokio::test]
async fn create_pr_branches_creates_branch_before_pr_request() {
    let mut mock_forge = MockForge::new();

    // Expect the branch to be created
    mock_forge
        .expect_create_release_branch()
        .times(1)
        .withf(|req: &CreateReleaseBranchRequest| {
            req.base_branch == "main"
                && req.release_branch == "releasaurus-release-main"
                && req.message.contains("test-pkg")
                && req.message.contains("v1.2.3")
        })
        .returning(|_| {
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        });

    let orchestrator = create_core(mock_forge, None, None);

    let releasable = ReleasablePackage {
        name: "test-pkg".to_string(),
        tag: Tag {
            name: "v1.2.3".to_string(),
            semver: Version::parse("1.2.3").unwrap(),
            ..Default::default()
        },
        notes: "Test release notes".to_string(),
        ..Default::default()
    };

    let grouped = orchestrator
        .release_pr_packages_by_branch(vec![releasable])
        .unwrap();

    let pr_requests = orchestrator.create_pr_branches(grouped).await.unwrap();

    assert_eq!(pr_requests.len(), 1);
    assert_eq!(pr_requests[0].base_branch, "main");
    assert_eq!(pr_requests[0].head_branch, "releasaurus-release-main");
}

#[tokio::test]
async fn create_pr_branches_includes_metadata_in_body() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_create_release_branch()
        .times(1)
        .returning(|_| {
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        });

    let orchestrator = create_core(mock_forge, None, None);

    let releasable = ReleasablePackage {
        name: "test-pkg".to_string(),
        tag: Tag {
            name: "v1.2.3".to_string(),
            semver: Version::parse("1.2.3").unwrap(),
            ..Default::default()
        },
        notes: "Test release notes".to_string(),
        ..Default::default()
    };

    let grouped = orchestrator
        .release_pr_packages_by_branch(vec![releasable])
        .unwrap();

    let pr_requests = orchestrator.create_pr_branches(grouped).await.unwrap();

    assert_eq!(pr_requests.len(), 1);

    let request = &pr_requests[0];
    // Verify metadata is in PR body
    assert!(request.body.contains("<!--"));
    assert!(request.body.contains("test-pkg"));
    assert!(request.body.contains("v1.2.3"));
    assert!(request.body.contains("Test release notes"));
    // Verify details tag is present and auto-opened for single package
    assert!(request.body.contains("<details open>"));
    assert!(request.body.contains("</details>"));
}

#[tokio::test]
async fn create_pr_branches_handles_multiple_packages_on_same_branch() {
    let mut mock_forge = MockForge::new();

    // Should only create one branch for multiple packages
    mock_forge
        .expect_create_release_branch()
        .times(1)
        .withf(|req: &CreateReleaseBranchRequest| {
            req.base_branch == "main"
                && req.release_branch == "releasaurus-release-main"
                && !req.message.contains("pkg-a")
                && !req.message.contains("pkg-b")
        })
        .returning(|_| {
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        });

    let pkg_configs = vec![
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
    ];

    let config = Config {
        separate_pull_requests: false,
        ..Default::default()
    };

    let orchestrator = create_core(mock_forge, Some(pkg_configs), Some(config));

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

    let pr_requests = orchestrator.create_pr_branches(grouped).await.unwrap();

    // Should create one PR for both packages
    assert_eq!(pr_requests.len(), 1);

    let request = &pr_requests[0];
    // Both packages should be in the body
    assert!(request.body.contains("pkg-a"));
    assert!(request.body.contains("pkg-b"));
    assert!(request.body.contains("v1.0.0"));
    assert!(request.body.contains("v2.0.0"));
    // Details should NOT be auto-opened when multiple packages
    assert!(request.body.contains("<details>"));
    assert!(!request.body.contains("<details open>"));
}

#[tokio::test]
async fn create_pr_branches_handles_separate_branches() {
    let mut mock_forge = MockForge::new();

    // Should create two separate branches
    mock_forge
        .expect_create_release_branch()
        .times(2)
        .withf(|req: &CreateReleaseBranchRequest| {
            req.base_branch == "main"
                && (req.release_branch == "releasaurus-release-main-pkg-a"
                    || req.release_branch == "releasaurus-release-main-pkg-b")
        })
        .returning(|_| {
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        });

    let pkg_configs = vec![
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
    ];

    let config = Config {
        separate_pull_requests: true,
        ..Default::default()
    };

    let orchestrator = create_core(mock_forge, Some(pkg_configs), Some(config));

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

    let pr_requests = orchestrator.create_pr_branches(grouped).await.unwrap();

    // Should create two separate PRs
    assert_eq!(pr_requests.len(), 2);

    // Each PR should have its own branch
    let branches: Vec<&String> =
        pr_requests.iter().map(|pr| &pr.head_branch).collect();
    assert!(branches.contains(&&"releasaurus-release-main-pkg-a".to_string()));
    assert!(branches.contains(&&"releasaurus-release-main-pkg-b".to_string()));
}

#[tokio::test]
async fn create_pr_branches_includes_file_changes() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_create_release_branch()
        .times(1)
        .withf(|req: &CreateReleaseBranchRequest| {
            // Should have at least the changelog file
            !req.file_changes.is_empty()
        })
        .returning(|_| {
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        });

    let orchestrator = create_core(mock_forge, None, None);

    let releasable = ReleasablePackage {
        name: "test-pkg".to_string(),
        tag: Tag {
            name: "v1.0.0".to_string(),
            semver: Version::parse("1.0.0").unwrap(),
            ..Default::default()
        },
        notes: "Release notes".to_string(),
        ..Default::default()
    };

    let grouped = orchestrator
        .release_pr_packages_by_branch(vec![releasable])
        .unwrap();

    let pr_requests = orchestrator.create_pr_branches(grouped).await.unwrap();

    assert_eq!(pr_requests.len(), 1);
}

#[tokio::test]
async fn create_pr_branches_uses_correct_title_format() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_create_release_branch()
        .times(1)
        .returning(|_| {
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        });

    let orchestrator = create_core(mock_forge, None, None);

    let releasable = ReleasablePackage {
        name: "test-pkg".to_string(),
        tag: Tag {
            name: "v1.2.3".to_string(),
            semver: Version::parse("1.2.3").unwrap(),
            ..Default::default()
        },
        notes: "Test release".to_string(),
        ..Default::default()
    };

    let grouped = orchestrator
        .release_pr_packages_by_branch(vec![releasable])
        .unwrap();

    let pr_requests = orchestrator.create_pr_branches(grouped).await.unwrap();

    assert_eq!(pr_requests.len(), 1);

    let request = &pr_requests[0];
    // Single package should have specific title format
    assert!(request.title.contains("chore(main): release"));
    assert!(request.title.contains("test-pkg"));
    assert!(request.title.contains("v1.2.3"));
}
