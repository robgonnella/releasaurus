//! Commit filtering tests.
//!
//! Tests for:
//! - skip_ci filtering behavior
//! - skip_chore filtering behavior
//! - skip_docs filtering behavior
//! - skip_test filtering behavior
//! - skip_style filtering behavior
//! - skip_refactor filtering behavior
//! - skip_perf filtering behavior
//! - skip_revert filtering behavior
//! - skip_miscellaneous filtering behavior
//! - include_author flag
//! - Combined filtering scenarios

use crate::{
    analyzer::{Analyzer, config::AnalyzerConfig, group, release},
    config::changelog::RewordedCommit,
    forge::request::ForgeCommit,
    orchestrator::config::CommitModifiers,
};
use semver::Version as SemVer;

/// Convenience constructor for test commits.
fn make_commit(id: &str, message: &str, timestamp: i64) -> ForgeCommit {
    ForgeCommit {
        id: id.to_string(),
        message: message.to_string(),
        timestamp,
        ..ForgeCommit::default()
    }
}

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
        make_commit("abc123", "feat: add new feature", 1000),
        make_commit("def456", "ci: update workflow", 2000),
        make_commit("ghi789", "ci: fix pipeline", 3000),
        make_commit("jkl012", "fix: bug fix", 4000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();
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
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "ci: update workflow", 2000),
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();
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
        make_commit("abc123", "feat: add new feature", 1000),
        make_commit("def456", "chore: update dependencies", 2000),
        make_commit("ghi789", "chore: cleanup code", 3000),
        make_commit("jkl012", "fix: bug fix", 4000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();
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
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "chore: cleanup", 2000),
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();
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
        make_commit("abc123", "feat: add new feature", 1000),
        make_commit("def456", "random commit without type", 2000),
        make_commit("ghi789", "another non-conventional commit", 3000),
        make_commit("jkl012", "fix: bug fix", 4000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();
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
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "random commit", 2000),
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();
    // Both commits should be included
    assert_eq!(release.commits.len(), 2);
}

#[test]
fn test_skip_docs_filters_docs_commits() {
    let config = AnalyzerConfig {
        skip_doc: true,
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
        make_commit("abc123", "feat: add new feature", 1000),
        make_commit("def456", "docs: update readme", 2000),
        make_commit("ghi789", "fix: bug fix", 3000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();
    // Should only have 2 commits (feat and fix), docs commit filtered out
    assert_eq!(release.commits.len(), 2);
    assert!(release.commits.iter().all(|c| c.group != group::Group::Doc));
}

#[test]
fn test_skip_docs_false_includes_docs_commits() {
    let config = AnalyzerConfig {
        skip_doc: false,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "docs: update readme", 2000),
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();
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
        make_commit("abc123", "feat: add new feature", 1000),
        make_commit("def456", "ci: update workflow", 2000),
        make_commit("ghi789", "chore: cleanup", 3000),
        make_commit("jkl012", "random commit", 4000),
        make_commit("mno345", "fix: fix bug", 5000),
        make_commit("pqr678", "docs: update readme", 6000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();
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

    let commits = vec![make_commit("abc123", "feat: new feature", 1000)];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();
    // Should have include_author set to true
    assert!(release.include_author);
}

#[test]
fn test_include_author_false_by_default() {
    let config = AnalyzerConfig::default();
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![make_commit("abc123", "feat: new feature", 1000)];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();
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
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "fix: bug fix", 2000),
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();
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
        make_commit("abc123", "ci: update workflow", 1000),
        make_commit("def456", "chore: cleanup", 2000),
        make_commit("ghi789", "random commit", 3000),
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
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "fix: bug fix", 2000),
        make_commit("ghi789", "feat: another feature", 3000),
        make_commit("jkl012", "fix: another fix", 4000),
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
        make_commit(
            "abc123def456789012345678901234567890abcd",
            "feat: should be skipped",
            1000,
        ),
        make_commit("def456", "fix: should be included", 2000),
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
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "fix: bug fix", 2000),
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
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "fix: bug fix", 2000),
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
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "fix: original message", 2000),
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

    let commits = vec![make_commit(
        "abc123def456789012345678901234567890abcd",
        "fix: original message",
        1000,
    )];

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

    let commits = vec![make_commit("abc123", "feat: original", 1000)];

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

    let commits =
        vec![make_commit("abc123", "fix: original was just a fix", 1000)];

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

    let commits = vec![make_commit("abc123", "fix: bug fix", 1000)];

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
        make_commit("abc123", "feat: first feature", 1000),
        make_commit("def456", "fix: original fix", 2000),
        make_commit("ghi789", "feat: will be skipped", 3000),
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    // Should have 2 commits (ghi789 skipped)
    assert_eq!(release.commits.len(), 2);
    assert_eq!(release.commits[0].id, "abc123");
    assert_eq!(release.commits[1].id, "def456");
    // Second commit should be reworded
    assert_eq!(release.commits[1].title, "upgraded from fix");
}

#[test]
fn test_skip_test_filters_test_commits() {
    let config = AnalyzerConfig {
        skip_test: true,
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
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "test: add unit tests", 2000),
        make_commit("ghi789", "test: add integration tests", 3000),
        make_commit("jkl012", "fix: bug fix", 4000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    assert_eq!(release.commits.len(), 2);
    assert!(
        release
            .commits
            .iter()
            .all(|c| c.group != group::Group::Test)
    );
}

#[test]
fn test_skip_test_false_includes_test_commits() {
    let config = AnalyzerConfig {
        skip_test: false,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "test: add unit tests", 2000),
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    assert_eq!(release.commits.len(), 2);
}

#[test]
fn test_skip_style_filters_style_commits() {
    let config = AnalyzerConfig {
        skip_style: true,
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
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "style: format code with prettier", 2000),
        make_commit("ghi789", "fix: bug fix", 3000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    assert_eq!(release.commits.len(), 2);
    assert!(
        release
            .commits
            .iter()
            .all(|c| c.group != group::Group::Style)
    );
}

#[test]
fn test_skip_style_false_includes_style_commits() {
    let config = AnalyzerConfig {
        skip_style: false,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "style: format code", 2000),
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    assert_eq!(release.commits.len(), 2);
}

#[test]
fn test_skip_refactor_filters_refactor_commits() {
    let config = AnalyzerConfig {
        skip_refactor: true,
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
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "refactor: simplify auth logic", 2000),
        make_commit("ghi789", "fix: bug fix", 3000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    assert_eq!(release.commits.len(), 2);
    assert!(
        release
            .commits
            .iter()
            .all(|c| c.group != group::Group::Refactor)
    );
}

#[test]
fn test_skip_refactor_false_includes_refactor_commits() {
    let config = AnalyzerConfig {
        skip_refactor: false,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "refactor: clean up code", 2000),
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    assert_eq!(release.commits.len(), 2);
}

#[test]
fn test_skip_perf_filters_perf_commits() {
    let config = AnalyzerConfig {
        skip_perf: true,
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
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "perf: optimize database queries", 2000),
        make_commit("ghi789", "fix: bug fix", 3000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    assert_eq!(release.commits.len(), 2);
    assert!(
        release
            .commits
            .iter()
            .all(|c| c.group != group::Group::Perf)
    );
}

#[test]
fn test_skip_perf_false_includes_perf_commits() {
    let config = AnalyzerConfig {
        skip_perf: false,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "perf: cache results", 2000),
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    assert_eq!(release.commits.len(), 2);
}

#[test]
fn test_skip_revert_filters_revert_commits() {
    let config = AnalyzerConfig {
        skip_revert: true,
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
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "revert: undo breaking changes", 2000),
        make_commit("ghi789", "fix: bug fix", 3000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    assert_eq!(release.commits.len(), 2);
    assert!(
        release
            .commits
            .iter()
            .all(|c| c.group != group::Group::Revert)
    );
}

#[test]
fn test_skip_revert_false_includes_revert_commits() {
    let config = AnalyzerConfig {
        skip_revert: false,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        make_commit("abc123", "feat: add feature", 1000),
        make_commit("def456", "revert: undo last commit", 2000),
    ];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    assert_eq!(release.commits.len(), 2);
}
