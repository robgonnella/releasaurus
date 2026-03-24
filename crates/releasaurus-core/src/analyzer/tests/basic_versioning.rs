//! Basic analyzer functionality tests.
//!
//! Tests for:
//! - Analyzer construction
//! - Empty commit handling
//! - First releases
//! - Version bumping (patch, minor, major)
//! - Tag prefix handling
//! - Multiple commits

use crate::{
    analyzer::{Analyzer, config::AnalyzerConfig, release},
    forge::request::ForgeCommit,
};
use semver::Version as SemVer;

#[test]
fn test_analyzer_new() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config).unwrap();
    assert_eq!(analyzer.config.tag_prefix, config.tag_prefix);
}

#[test]
fn test_analyze_empty_commits() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config).unwrap();
    let result = analyzer.analyze(vec![], None).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_analyze_first_release_no_tag() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: add new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "fix: fix bug".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, None).unwrap();

    let release = result.unwrap();
    assert_eq!(release.tag.semver, SemVer::parse("0.1.0").unwrap());
    assert_eq!(release.commits.len(), 2);
}

#[test]
fn test_analyze_with_current_tag_patch_bump() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..release::Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "fix: fix critical bug".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    assert_eq!(release.tag.semver, SemVer::parse("1.0.1").unwrap());
}

#[test]
fn test_analyze_with_current_tag_minor_bump() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..release::Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat: add new feature".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    assert_eq!(release.tag.semver, SemVer::parse("1.1.0").unwrap());
}

#[test]
fn test_analyze_with_current_tag_major_bump() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..release::Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat!: breaking change".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    assert_eq!(release.tag.semver, SemVer::parse("2.0.0").unwrap());
}

#[test]
fn test_analyze_with_tag_prefix() {
    let config = AnalyzerConfig {
        tag_prefix: Some("v".to_string()),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat: new feature".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, None).unwrap();

    let release = result.unwrap();
    assert_eq!(release.tag.name, "v0.1.0");
}

#[test]
fn test_analyze_generates_release_link() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat!: breaking change".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, None).unwrap();

    let release = result.unwrap();
    assert_eq!(release.commits.len(), 1);
}

#[test]
fn test_analyze_multiple_commits() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..release::Tag::default()
    };

    let commits = vec![
        ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: feature one".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "feat: feature two".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "ghi789".to_string(),
            message: "fix: bug fix".to_string(),
            timestamp: 3000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    assert_eq!(release.commits.len(), 3);
    // Should bump minor due to features
    assert_eq!(release.tag.semver, SemVer::parse("1.1.0").unwrap());
}

#[test]
fn test_chore_only_with_no_tag() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config).unwrap();

    // Only a chore commit - should still create a first release (0.1.0)
    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "chore: update dependencies".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, None).unwrap();

    // Chore commits still trigger a first release
    let release = result.unwrap();
    assert_eq!(release.tag.semver, SemVer::parse("0.1.0").unwrap());
    assert_eq!(release.commits.len(), 1);
}

#[test]
fn test_chore_only_with_existing_tag() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..release::Tag::default()
    };

    // Only a chore commit with existing tag
    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "chore: update dependencies".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    // Chore commits bump patch version (per next_version crate behavior)
    let release = result.unwrap();
    assert_eq!(release.tag.semver, SemVer::parse("1.0.1").unwrap());
    assert_eq!(release.commits.len(), 1);
}
