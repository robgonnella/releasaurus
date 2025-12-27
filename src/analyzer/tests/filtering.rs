//! Commit filtering tests.
//!
//! Tests for:
//! - skip_ci filtering behavior
//! - skip_chore filtering behavior
//! - skip_miscellaneous filtering behavior
//! - include_author flag
//! - Combined filtering scenarios

use crate::{
    analyzer::{Analyzer, config::AnalyzerConfig, group, release},
    forge::request::ForgeCommit,
};
use semver::Version as SemVer;

#[test]
fn test_skip_ci_filters_ci_commits() {
    let config = AnalyzerConfig {
        skip_ci: true,
        ..AnalyzerConfig::default()
    };
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
            message: "feat: add new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "ci: update workflow".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "ghi789".to_string(),
            message: "ci: fix pipeline".to_string(),
            timestamp: 3000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "jkl012".to_string(),
            message: "fix: bug fix".to_string(),
            timestamp: 4000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    // Should only have 2 commits (feat and fix), ci commits filtered out
    assert_eq!(release.commits.len(), 2);
    assert!(release.commits.iter().all(|c| c.group != group::Group::Ci));
}

#[test]
fn test_skip_ci_false_includes_ci_commits() {
    let config = AnalyzerConfig {
        skip_ci: false,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: add feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "ci: update workflow".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, None).unwrap();

    let release = result.unwrap();
    // Both commits should be included
    assert_eq!(release.commits.len(), 2);
}

#[test]
fn test_skip_chore_filters_chore_commits() {
    let config = AnalyzerConfig {
        skip_chore: true,
        ..AnalyzerConfig::default()
    };
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
            message: "feat: add new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "chore: update dependencies".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "ghi789".to_string(),
            message: "chore: cleanup code".to_string(),
            timestamp: 3000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "jkl012".to_string(),
            message: "fix: bug fix".to_string(),
            timestamp: 4000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    // Should only have 2 commits (feat and fix), chore commits filtered out
    assert_eq!(release.commits.len(), 2);
    assert!(
        release
            .commits
            .iter()
            .all(|c| c.group != group::Group::Chore)
    );
}

#[test]
fn test_skip_chore_false_includes_chore_commits() {
    let config = AnalyzerConfig {
        skip_chore: false,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: add feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "chore: cleanup".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, None).unwrap();

    let release = result.unwrap();
    // Both commits should be included
    assert_eq!(release.commits.len(), 2);
}

#[test]
fn test_skip_miscellaneous_filters_non_conventional_commits() {
    let config = AnalyzerConfig {
        skip_miscellaneous: true,
        ..AnalyzerConfig::default()
    };
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
            message: "feat: add new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "random commit without type".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "ghi789".to_string(),
            message: "another non-conventional commit".to_string(),
            timestamp: 3000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "jkl012".to_string(),
            message: "fix: bug fix".to_string(),
            timestamp: 4000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    // Should only have 2 commits (feat and fix), miscellaneous filtered out
    assert_eq!(release.commits.len(), 2);
    assert!(
        release
            .commits
            .iter()
            .all(|c| c.group != group::Group::Miscellaneous)
    );
}

#[test]
fn test_skip_miscellaneous_false_includes_non_conventional_commits() {
    let config = AnalyzerConfig {
        skip_miscellaneous: false,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: add feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "random commit".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, None).unwrap();

    let release = result.unwrap();
    // Both commits should be included
    assert_eq!(release.commits.len(), 2);
}

#[test]
fn test_skip_multiple_types_combined() {
    let config = AnalyzerConfig {
        skip_ci: true,
        skip_chore: true,
        skip_miscellaneous: true,
        ..AnalyzerConfig::default()
    };
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
            message: "feat: add new feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "ci: update workflow".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "ghi789".to_string(),
            message: "chore: cleanup".to_string(),
            timestamp: 3000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "jkl012".to_string(),
            message: "random commit".to_string(),
            timestamp: 4000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "mno345".to_string(),
            message: "fix: fix bug".to_string(),
            timestamp: 5000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "pqr678".to_string(),
            message: "docs: update readme".to_string(),
            timestamp: 6000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    // Should only have 3 commits (feat, fix, docs)
    assert_eq!(release.commits.len(), 3);
    assert!(release.commits.iter().all(|c| c.group != group::Group::Ci));
    assert!(
        release
            .commits
            .iter()
            .all(|c| c.group != group::Group::Chore)
    );
    assert!(
        release
            .commits
            .iter()
            .all(|c| c.group != group::Group::Miscellaneous)
    );
}

#[test]
fn test_include_author_sets_release_flag() {
    let config = AnalyzerConfig {
        include_author: true,
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
    // Should have include_author set to true
    assert!(release.include_author);
}

#[test]
fn test_include_author_false_by_default() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat: new feature".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, None).unwrap();

    let release = result.unwrap();
    // Should have include_author set to false by default
    assert!(!release.include_author);
}

#[test]
fn test_skip_ci_with_no_ci_commits() {
    let config = AnalyzerConfig {
        skip_ci: true,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: add feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "fix: bug fix".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, None).unwrap();

    let release = result.unwrap();
    assert_eq!(release.commits.len(), 2);
}

#[test]
fn test_skip_all_types_results_in_no_release() {
    let config = AnalyzerConfig {
        skip_ci: true,
        skip_chore: true,
        skip_miscellaneous: true,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..release::Tag::default()
    };

    // Only commits that will be filtered out
    let commits = vec![
        ForgeCommit {
            id: "abc123".to_string(),
            message: "ci: update workflow".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "chore: cleanup".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "ghi789".to_string(),
            message: "random commit".to_string(),
            timestamp: 3000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    // Should result in no release since all commits are filtered
    assert!(result.is_none());
}
