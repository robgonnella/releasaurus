//! Custom version increment rules tests.
//!
//! Tests for:
//! - breaking_always_increment_major flag behavior
//! - features_always_increment_minor flag behavior
//! - custom_major_increment_regex configuration
//! - custom_minor_increment_regex configuration
//! - Combined flag scenarios
//! - Non-conventional commit matching

use regex::Regex;
use semver::Version as SemVer;

use crate::{
    analyzer::{Analyzer, config::AnalyzerConfig},
    config::versioning::{Group, NAMED_PARSERS},
    forge::request::{ForgeCommit, Tag},
};

/// `custom_major_increment_regex` does not only bump the version - a commit
/// it matches is treated as breaking outright, which means the `Breaking`
/// changelog group and the `breaking` flag the default body template gates
/// its `[**breaking**]` marker on.
///
/// This is the whole reason the flag is set during commit parsing rather than
/// left to the version updater, and it is the half a version-only assertion
/// misses: the bump comes from `next_version` reading the raw messages, so it
/// happens either way and cannot distinguish the two designs.
///
/// `breaking.pattern` resolves into this same field (see
/// `resolve_versioning`), so this covers both spellings.
#[test]
fn test_custom_major_regex_marks_commit_breaking_and_groups_it() {
    let config = AnalyzerConfig {
        custom_major_increment_regex: Some(Regex::new("^breaking").unwrap()),
        ..AnalyzerConfig::default()
    };
    let breaking_title =
        NAMED_PARSERS.get(&Group::Breaking).unwrap().group_title();
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..Tag::default()
    };

    let commits = vec![
        ForgeCommit {
            id: "abc123".to_string(),
            message: "breaking: drop the v1 endpoint".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "chore: tidy up".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    let matched = release
        .commits
        .iter()
        .find(|c| c.id == "abc123")
        .expect("matching commit should survive analysis");

    assert!(
        matched.breaking,
        "a matching commit must be flagged breaking"
    );
    assert_eq!(matched.group, breaking_title);
    assert_eq!(release.tag.semver, SemVer::parse("2.0.0").unwrap());

    // A commit the pattern does not match is untouched, so the regex cannot
    // be quietly matching everything.
    let other = release
        .commits
        .iter()
        .find(|c| c.id == "def456")
        .expect("non-matching commit should survive analysis");

    assert!(!other.breaking);
    assert_ne!(other.group, breaking_title);
}

/// Both increment flags reach the analyzer as `Option<bool>` - the resolver
/// passes the config tiers through unresolved - so the default is applied
/// deep in `Context::create_version_updater`, from
/// `DEFAULT_BREAKING_ALWAYS_INCREMENT_MAJOR` and
/// `DEFAULT_FEAT_ALWAYS_INCREMENT_MINOR`. These two tests cover the unset
/// path: every other test in this file sets the flags explicitly, so nothing
/// else would catch the runtime default drifting away from the value the JSON
/// schema documents. Flipping either constant fails the assertion below.
#[test]
fn test_breaking_always_increments_major_when_unset() {
    let config = AnalyzerConfig {
        breaking_always_increment_major: None,
        ..AnalyzerConfig::default()
    };

    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat!: breaking change".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    // Contrast with `test_breaking_always_increment_major_disabled`, where the
    // same 0.x commit only reaches 0.2.0.
    assert_eq!(release.tag.semver, SemVer::parse("1.0.0").unwrap());
}

#[test]
fn test_features_always_increment_minor_when_unset() {
    let config = AnalyzerConfig {
        features_always_increment_minor: None,
        ..AnalyzerConfig::default()
    };

    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat: a new feature".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    assert_eq!(release.tag.semver, SemVer::parse("0.2.0").unwrap());
}

#[test]
fn test_breaking_always_increment_major_disabled() {
    let config = AnalyzerConfig {
        breaking_always_increment_major: Some(false),
        ..AnalyzerConfig::default()
    };

    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
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

    // In 0.x versions with breaking_always_increment_major=false,
    // breaking changes bump minor instead of major
    assert_eq!(release.tag.semver, SemVer::parse("0.2.0").unwrap());
}

#[test]
fn test_custom_major_regex_works_with_breaking_syntax() {
    let major_regex = Regex::new("MAJOR").unwrap();
    let config = AnalyzerConfig {
        custom_major_increment_regex: Some(major_regex),
        ..AnalyzerConfig::default()
    };

    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..Tag::default()
    };

    // Conventional breaking syntax still works even with custom regex
    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat!: breaking change".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
    let release = result.unwrap();

    // Breaking syntax still triggers major bump (custom regex is additive)
    assert_eq!(release.tag.semver, SemVer::parse("1.0.0").unwrap());
}

#[test]
fn test_custom_major_increment_regex() {
    let doc_regex = Regex::new("doc").unwrap();
    let config = AnalyzerConfig {
        custom_major_increment_regex: Some(doc_regex),
        ..AnalyzerConfig::default()
    };

    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "doc: this should bump major".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
    let release = result.unwrap();

    // Custom regex matches "doc" in commit message, bumps major
    assert_eq!(release.tag.semver, SemVer::parse("1.0.0").unwrap());
}

#[test]
fn test_features_always_increment_minor_disabled() {
    let config = AnalyzerConfig {
        features_always_increment_minor: Some(false),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat: new feature".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
    let release = result.unwrap();

    // In 0.x versions with features_always_increment_minor=false,
    // features bump patch instead of minor
    assert_eq!(release.tag.semver, SemVer::parse("0.1.1").unwrap());
}

#[test]
fn test_custom_minor_increment_regex() {
    let ci_regex = Regex::new(r"^ci").unwrap();
    let config = AnalyzerConfig {
        custom_minor_increment_regex: Some(ci_regex),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "ci: this should bump minor".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
    let release = result.unwrap();

    // Custom regex matches "ci" in commit message, bumps minor
    assert_eq!(release.tag.semver, SemVer::parse("0.2.0").unwrap());
}

#[test]
fn test_custom_minor_regex_works_with_feat_syntax() {
    let ci_regex = Regex::new(r"ci").unwrap();
    let config = AnalyzerConfig {
        custom_minor_increment_regex: Some(ci_regex),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat: new feature".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
    let release = result.unwrap();

    // Feat syntax still triggers minor bump (custom regex is additive)
    assert_eq!(release.tag.semver, SemVer::parse("0.2.0").unwrap());
}

#[test]
fn test_both_boolean_flags_disabled_minor_bump() {
    let config = AnalyzerConfig {
        features_always_increment_minor: Some(false),
        breaking_always_increment_major: Some(false),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..Tag::default()
    };

    // With both flags disabled, only minor bump should occur
    let commits = vec![
        ForgeCommit {
            id: "abc123".to_string(),
            message: "feat!: breaking feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "def456".to_string(),
            message: "feat: regular feature".to_string(),
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

    // With both flags disabled, only minor bump
    assert_eq!(release.tag.semver, SemVer::parse("0.2.0").unwrap());
}

#[test]
fn test_both_boolean_flags_disabled_patch_bump() {
    let config = AnalyzerConfig {
        features_always_increment_minor: Some(false),
        breaking_always_increment_major: Some(false),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..Tag::default()
    };

    // With both flags disabled, only patch bump should occur
    let commits = vec![
        ForgeCommit {
            id: "def456".to_string(),
            message: "feat: regular feature".to_string(),
            timestamp: 1000,
            ..ForgeCommit::default()
        },
        ForgeCommit {
            id: "ghi789".to_string(),
            message: "fix: bug fix".to_string(),
            timestamp: 2000,
            ..ForgeCommit::default()
        },
    ];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
    let release = result.unwrap();

    // With both flags disabled, only patch bump
    assert_eq!(release.tag.semver, SemVer::parse("0.1.1").unwrap());
}

#[test]
fn test_custom_regex_matches_non_conventional_commit() {
    let wow_regex = Regex::new(r"wow").unwrap();
    let config = AnalyzerConfig {
        custom_major_increment_regex: Some(wow_regex),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..Tag::default()
    };

    // Non-conventional commit message that matches custom regex
    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "wow: complete rewrite of core functionality".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();
    let release = result.unwrap();

    // Custom regex matches "wow" and triggers major bump
    assert_eq!(release.tag.semver, SemVer::parse("1.0.0").unwrap());
}
