//! Tests for package analysis functionality.
//!
//! Tests for:
//! - Analyzing packages with commits
//! - Analyzing packages with existing tags
//! - Handling packages with no commits

use super::common::*;
use crate::{
    analyzer::release::Tag,
    forge::{request::ForgeCommitBuilder, traits::MockForge},
};

#[test]
fn analyze_packages_produces_analyzed_packages() {
    let mock_forge = MockForge::new();
    let orchestrator = create_core(mock_forge, None, None);

    let commits = vec![
        ForgeCommitBuilder::default()
            .id("commit1")
            .short_id("c1")
            .message("feat: new feature")
            .timestamp(1000)
            .files(vec!["src/main.rs".to_string()])
            .build()
            .unwrap(),
        ForgeCommitBuilder::default()
            .id("commit2")
            .short_id("c2")
            .message("fix: bug fix")
            .timestamp(2000)
            .files(vec!["src/lib.rs".to_string()])
            .build()
            .unwrap(),
    ];

    let prepared = vec![PreparedPackage {
        name: "test-pkg".to_string(),
        current_tag: None,
        commits,
    }];

    let analyzed = orchestrator.analyze_packages(prepared).unwrap();
    assert_eq!(analyzed.len(), 1);
    assert_eq!(analyzed[0].name, "test-pkg");
    analyzed[0].release.as_ref().unwrap();
}

#[test]
fn analyze_packages_with_existing_tag() {
    let mock_forge = MockForge::new();
    let orchestrator = create_core(mock_forge, None, None);

    let commits = vec![
        ForgeCommitBuilder::default()
            .id("new-commit")
            .short_id("new")
            .message("fix: post-release fix")
            .timestamp(3000)
            .files(vec!["src/main.rs".to_string()])
            .build()
            .unwrap(),
    ];

    let current_tag = Some(Tag {
        semver: Version::parse("1.0.0").unwrap(),
        timestamp: Some(2000),
        ..Default::default()
    });

    let prepared = vec![PreparedPackage {
        name: "test-pkg".to_string(),
        current_tag,
        commits,
    }];

    let analyzed = orchestrator.analyze_packages(prepared).unwrap();
    assert_eq!(analyzed.len(), 1);

    if let Some(release) = &analyzed[0].release {
        // Should bump from 1.0.0 to 1.0.1 for a fix
        assert_eq!(release.tag.semver.major, 1);
        assert_eq!(release.tag.semver.minor, 0);
        assert_eq!(release.tag.semver.patch, 1);
    }
}

#[test]
fn analyze_packages_returns_none_when_no_commits() {
    let mock_forge = MockForge::new();
    let orchestrator = create_core(mock_forge, None, None);

    let prepared = vec![PreparedPackage {
        name: "test-pkg".to_string(),
        current_tag: None,
        commits: vec![],
    }];

    let analyzed = orchestrator.analyze_packages(prepared).unwrap();
    assert_eq!(analyzed.len(), 1);
    assert!(analyzed[0].release.is_none());
}
