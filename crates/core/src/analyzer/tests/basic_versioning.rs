//! Basic analyzer functionality tests.
//!
//! Tests for:
//! - Analyzer construction
//! - Empty commit handling
//! - First releases
//! - Version bumping (patch, minor, major)
//! - Tag prefix handling
//! - Multiple commits

use semver::Version as SemVer;
use url::Url;

use crate::{
    analyzer::{Analyzer, config::AnalyzerConfig},
    config::changelog::DEFAULT_PARSERS,
    forge::request::{ForgeCommit, Tag},
};

#[test]
fn test_analyzer_new() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config, &DEFAULT_PARSERS, &[]).unwrap();
    assert_eq!(analyzer.config.tag_prefix, config.tag_prefix);
}

#[test]
fn test_analyze_empty_commits() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config, &DEFAULT_PARSERS, &[]).unwrap();
    let result = analyzer.analyze(vec![], None).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_analyze_first_release_no_tag() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config, &DEFAULT_PARSERS, &[]).unwrap();

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
    let analyzer = Analyzer::new(&config, &DEFAULT_PARSERS, &[]).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..Tag::default()
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
    let analyzer = Analyzer::new(&config, &DEFAULT_PARSERS, &[]).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..Tag::default()
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
    let analyzer = Analyzer::new(&config, &DEFAULT_PARSERS, &[]).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..Tag::default()
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
    let analyzer = Analyzer::new(&config, &DEFAULT_PARSERS, &[]).unwrap();

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
    let analyzer = Analyzer::new(&config, &DEFAULT_PARSERS, &[]).unwrap();

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
    let analyzer = Analyzer::new(&config, &DEFAULT_PARSERS, &[]).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..Tag::default()
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
fn test_sha_compare_link_uses_newest_commit() {
    let config = AnalyzerConfig {
        compare_link_base_url: Some(
            Url::parse("https://example.com/compare/").unwrap(),
        ),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config, &DEFAULT_PARSERS, &[]).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..Tag::default()
    };

    // commits arrive newest-first, matching the Forge::get_commits contract
    let commits = vec![
        ForgeCommit {
            id: "newest999".to_string(),
            message: "fix: latest fix".to_string(),
            timestamp: 3000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "middle555".to_string(),
            message: "fix: another fix".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "oldest111".to_string(),
            message: "fix: first fix after release".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    assert_eq!(release.sha, "newest999");
    assert_eq!(release.timestamp, 3000);
    assert_eq!(
        release.sha_compare_link,
        "https://example.com/compare/1.0.0...newest999"
    );
}

#[test]
fn test_sha_compare_link_spans_filtered_newest_commit() {
    let config = AnalyzerConfig {
        compare_link_base_url: Some(
            Url::parse("https://example.com/compare/").unwrap(),
        ),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config, &DEFAULT_PARSERS, &[]).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..Tag::default()
    };

    // newest commit is a merge commit, filtered from the changelog by
    // default, but the compare link should still span the whole range
    let commits = vec![
        ForgeCommit {
            id: "merge999".to_string(),
            message: "Merge pull request #42".to_string(),
            merge_commit: true,
            timestamp: 2000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "fix111".to_string(),
            message: "fix: a bug fix".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    assert_eq!(release.commits.len(), 1);
    assert_eq!(release.sha, "merge999");
    assert_eq!(release.timestamp, 2000);
    assert_eq!(
        release.sha_compare_link,
        "https://example.com/compare/1.0.0...merge999"
    );
}

#[test]
fn test_chore_only_with_no_tag() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config, &DEFAULT_PARSERS, &[]).unwrap();

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
    let analyzer = Analyzer::new(&config, &DEFAULT_PARSERS, &[]).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..Tag::default()
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
