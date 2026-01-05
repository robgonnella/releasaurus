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
    cli::CommitModifiers,
    config::changelog::RewordedCommit,
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

#[test]
fn test_skip_shas_filters_commits_by_sha() {
    let config = AnalyzerConfig {
        commit_modifiers: CommitModifiers {
            skip_shas: vec!["def456".to_string(), "ghi789".to_string()],
            reword: vec![],
        },
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
        ForgeCommit {
            id: "ghi789".to_string(),
            message: "feat: another feature".to_string(),
            timestamp: 3000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "jkl012".to_string(),
            message: "fix: another fix".to_string(),
            timestamp: 4000,
            ..ForgeCommit::default()
        },
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    // Should only have 2 commits (abc123 and jkl012)
    assert_eq!(release.commits.len(), 2);
    assert_eq!(release.commits[0].id, "abc123");
    assert_eq!(release.commits[1].id, "jkl012");
}

#[test]
fn test_skip_shas_matches_by_prefix() {
    let config = AnalyzerConfig {
        commit_modifiers: CommitModifiers {
            skip_shas: vec!["abc123".to_string()],
            reword: vec![],
        },
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        ForgeCommit {
            id: "abc123def456789012345678901234567890abcd".to_string(),
            message: "feat: should be skipped".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "fix: should be included".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    // Should only have 1 commit (def456) - full SHA starting with abc123 was skipped
    assert_eq!(release.commits.len(), 1);
    assert_eq!(release.commits[0].id, "def456");
}

#[test]
fn test_skip_shas_with_no_matching_commits() {
    let config = AnalyzerConfig {
        commit_modifiers: CommitModifiers {
            skip_shas: vec!["nonexistent".to_string()],
            reword: vec![],
        },
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

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    assert_eq!(release.commits.len(), 2);
}

#[test]
fn test_skip_shas_all_commits_results_in_no_release() {
    let config = AnalyzerConfig {
        commit_modifiers: CommitModifiers {
            skip_shas: vec!["abc123".to_string(), "def456".to_string()],
            reword: vec![],
        },
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

    assert!(result.is_none());
}

#[test]
fn test_reword_changes_commit_message() {
    let config = AnalyzerConfig {
        commit_modifiers: CommitModifiers {
            skip_shas: vec![],
            reword: vec![RewordedCommit {
                sha: "def456".to_string(),
                message: "fix: corrected bug description".to_string(),
            }],
        },
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
            message: "fix: original message".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    assert_eq!(release.commits.len(), 2);
    assert_eq!(release.commits[1].id, "def456");
    assert_eq!(
        release.commits[1].raw_message,
        "fix: corrected bug description"
    );
    assert_eq!(release.commits[1].title, "corrected bug description");
}

#[test]
fn test_reword_matches_by_prefix() {
    let config = AnalyzerConfig {
        commit_modifiers: CommitModifiers {
            skip_shas: vec![],
            reword: vec![RewordedCommit {
                sha: "abc123".to_string(),
                message: "feat: reworded message".to_string(),
            }],
        },
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![ForgeCommit {
        id: "abc123def456789012345678901234567890abcd".to_string(),
        message: "fix: original message".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    // Full SHA starting with abc123 should be reworded
    assert_eq!(release.commits[0].raw_message, "feat: reworded message");
    assert_eq!(release.commits[0].title, "reworded message");
}

#[test]
fn test_reword_with_multiline_message() {
    let config = AnalyzerConfig {
        commit_modifiers: CommitModifiers {
            skip_shas: vec![],
            reword: vec![RewordedCommit {
                sha: "abc123".to_string(),
                message: "feat: new title\n\nDetailed body content".to_string(),
            }],
        },
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat: original".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    assert_eq!(release.commits[0].title, "new title");
    assert_eq!(
        release.commits[0].body,
        Some("Detailed body content".to_string())
    );
}

#[test]
fn test_reword_changes_version_calculation() {
    let config = AnalyzerConfig {
        commit_modifiers: CommitModifiers {
            skip_shas: vec![],
            reword: vec![RewordedCommit {
                sha: "abc123".to_string(),
                message: "feat: upgraded to feature".to_string(),
            }],
        },
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..release::Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "fix: original was just a fix".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    // Should be minor bump (1.1.0) because reworded to feat, not patch (1.0.1)
    assert_eq!(release.tag.semver, SemVer::parse("1.1.0").unwrap());
}

#[test]
fn test_reword_with_no_matching_commits() {
    let config = AnalyzerConfig {
        commit_modifiers: CommitModifiers {
            skip_shas: vec![],
            reword: vec![RewordedCommit {
                sha: "nonexistent".to_string(),
                message: "feat: new message".to_string(),
            }],
        },
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "fix: bug fix".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    // Original message should be preserved
    assert_eq!(release.commits[0].raw_message, "fix: bug fix");
    assert_eq!(release.commits[0].title, "bug fix");
}

#[test]
fn test_skip_shas_and_reword_combined() {
    let config = AnalyzerConfig {
        commit_modifiers: CommitModifiers {
            skip_shas: vec!["ghi789".to_string()],
            reword: vec![RewordedCommit {
                sha: "def456".to_string(),
                message: "feat: upgraded from fix".to_string(),
            }],
        },
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        ForgeCommit {
            id: "abc123".to_string(),
            message: "feat: first feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "fix: original fix".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "ghi789".to_string(),
            message: "feat: will be skipped".to_string(),
            timestamp: 3000,
            ..ForgeCommit::default()
        },
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    // Should have 2 commits (ghi789 skipped)
    assert_eq!(release.commits.len(), 2);
    assert_eq!(release.commits[0].id, "abc123");
    assert_eq!(release.commits[1].id, "def456");
    // Second commit should be reworded
    assert_eq!(release.commits[1].title, "upgraded from fix");
}
