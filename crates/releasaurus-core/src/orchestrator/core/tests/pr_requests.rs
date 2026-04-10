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
        request::{Commit, CreateReleaseBranchRequest, PullRequest},
        traits::MockForge,
    },
    orchestrator::tests::common::{PrBodyInput, make_pr_body},
};

#[tokio::test]
async fn create_pr_branches_creates_branch_before_pr_request() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    mock_forge
        .expect_get_open_release_pr()
        .returning(|_| Ok(None));

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
        .await
        .unwrap();

    let pr_requests = orchestrator.create_pr_branches(grouped).await.unwrap();

    assert_eq!(pr_requests.len(), 1);
    assert_eq!(pr_requests[0].request.base_branch, "main");
    assert_eq!(
        pr_requests[0].request.head_branch,
        "releasaurus-release-main"
    );
}

#[tokio::test]
async fn create_pr_branches_includes_metadata_in_body() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    mock_forge
        .expect_get_open_release_pr()
        .returning(|_| Ok(None));

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
        .await
        .unwrap();

    let pr_requests = orchestrator.create_pr_branches(grouped).await.unwrap();

    assert_eq!(pr_requests.len(), 1);

    let request = &pr_requests[0];
    // Verify metadata is in PR body
    assert!(request.request.body.contains("<!--"));
    assert!(request.request.body.contains("test-pkg"));
    assert!(request.request.body.contains("v1.2.3"));
    assert!(request.request.body.contains("Test release notes"));
    // Verify details tag is present and auto-opened for single package
    assert!(request.request.body.contains("<details open>"));
    assert!(request.request.body.contains("</details>"));
}

#[tokio::test]
async fn create_pr_branches_uses_sha_compare_link() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    mock_forge
        .expect_get_open_release_pr()
        .returning(|_| Ok(None));

    mock_forge
        .expect_create_release_branch()
        .times(1)
        .returning(|_| {
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        });

    let core = create_core(mock_forge, None, None);

    let tag_compare_link = "tag_compare_link";
    let sha_compare_link = "sha_compare_link";

    let releasable = ReleasablePackage {
        name: "test-pkg".to_string(),
        tag: Tag {
            name: "v1.2.3".to_string(),
            semver: Version::parse("1.2.3").unwrap(),
            ..Default::default()
        },
        notes: format!("Test release notes\n\n{tag_compare_link}"),
        tag_compare_link: tag_compare_link.into(),
        sha_compare_link: sha_compare_link.into(),
        ..Default::default()
    };

    let grouped = core
        .release_pr_packages_by_branch(vec![releasable])
        .await
        .unwrap();

    let pr_requests = core.create_pr_branches(grouped).await.unwrap();

    assert_eq!(pr_requests.len(), 1);

    let request = &pr_requests[0];

    // Verify metadata is in PR body
    assert!(request.request.body.contains("<!--"));
    assert!(request.request.body.contains("test-pkg"));
    assert!(request.request.body.contains("v1.2.3"));
    assert!(request.request.body.contains("Test release notes"));
    // should have both version of compare links since we still need to
    // use tag_compare_link in PR metadata
    assert!(request.request.body.contains(tag_compare_link));
    assert!(request.request.body.contains(sha_compare_link));
    // Verify details tag is present and auto-opened for single package
    assert!(request.request.body.contains("<details open>"));
    assert!(request.request.body.contains("</details>"));
}

#[tokio::test]
async fn create_pr_branches_handles_multiple_packages_on_same_branch() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    mock_forge
        .expect_get_open_release_pr()
        .returning(|_| Ok(None));

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
        .await
        .unwrap();

    let pr_requests = orchestrator.create_pr_branches(grouped).await.unwrap();

    // Should create one PR for both packages
    assert_eq!(pr_requests.len(), 1);

    let request = &pr_requests[0];
    // Both packages should be in the body
    assert!(request.request.body.contains("pkg-a"));
    assert!(request.request.body.contains("pkg-b"));
    assert!(request.request.body.contains("v1.0.0"));
    assert!(request.request.body.contains("v2.0.0"));
    // Details should NOT be auto-opened when multiple packages
    assert!(request.request.body.contains("<details>"));
    assert!(!request.request.body.contains("<details open>"));
}

#[tokio::test]
async fn create_pr_branches_handles_separate_branches() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    mock_forge
        .expect_get_open_release_pr()
        .returning(|_| Ok(None));

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
        .await
        .unwrap();

    let pr_requests = orchestrator.create_pr_branches(grouped).await.unwrap();

    // Should create two separate PRs
    assert_eq!(pr_requests.len(), 2);

    // Each PR should have its own branch
    let branches: Vec<&String> = pr_requests
        .iter()
        .map(|pr| &pr.request.head_branch)
        .collect();
    assert!(branches.contains(&&"releasaurus-release-main-pkg-a".to_string()));
    assert!(branches.contains(&&"releasaurus-release-main-pkg-b".to_string()));
}

#[tokio::test]
async fn create_pr_branches_includes_file_changes() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    mock_forge
        .expect_get_open_release_pr()
        .returning(|_| Ok(None));

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
        .await
        .unwrap();

    let pr_requests = orchestrator.create_pr_branches(grouped).await.unwrap();

    assert_eq!(pr_requests.len(), 1);
}

#[tokio::test]
async fn create_pr_branches_uses_correct_title_format() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    mock_forge
        .expect_get_open_release_pr()
        .returning(|_| Ok(None));

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
        .await
        .unwrap();

    let pr_requests = orchestrator.create_pr_branches(grouped).await.unwrap();

    assert_eq!(pr_requests.len(), 1);

    let request = &pr_requests[0];
    // Single package should have specific title format
    assert!(request.request.title.contains("chore(main): release"));
    assert!(request.request.title.contains("test-pkg"));
    assert!(request.request.title.contains("v1.2.3"));
}

#[tokio::test]
async fn create_pr_branches_handles_existing_pr_body_sections() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_merged_release_pr()
        .returning(|_| Ok(None));

    let existing_body = make_pr_body(&PrBodyInput {
        pkg: "test-pkg",
        tag: "v1.2.2",
        notes: "Old release notes must not appear",
        tag_link: "old-tag-link",
        sha_link: "old-sha-link",
        header: "My custom header",
        footer: "My custom footer",
    });

    mock_forge.expect_get_open_release_pr().returning(move |_| {
        Ok(Some(PullRequest {
            number: 42,
            sha: "old-sha".to_string(),
            body: existing_body.clone(),
        }))
    });

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
        notes: "Freshly generated release notes".to_string(),
        ..Default::default()
    };

    let grouped = orchestrator
        .release_pr_packages_by_branch(vec![releasable])
        .await
        .unwrap();

    let results = orchestrator.create_pr_branches(grouped).await.unwrap();

    assert_eq!(results.len(), 1);
    let body = &results[0].request.body;
    // header and footer are preserved across re-runs
    assert!(body.contains("My custom header"));
    assert!(body.contains("My custom footer"));
    // notes are regenerated; old notes must not bleed through
    assert!(body.contains("Freshly generated release notes"));
    assert!(!body.contains("Old release notes must not appear"));
}
