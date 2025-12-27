//! Custom version increment rules tests.
//!
//! Tests for:
//! - breaking_always_increment_major flag behavior
//! - features_always_increment_minor flag behavior
//! - custom_major_increment_regex configuration
//! - custom_minor_increment_regex configuration
//! - Combined flag scenarios
//! - Non-conventional commit matching

use crate::{
    analyzer::{Analyzer, config::AnalyzerConfig, release},
    forge::request::ForgeCommit,
};
use semver::Version as SemVer;

#[test]
fn test_breaking_always_increment_major_disabled() {
    let config = AnalyzerConfig {
        breaking_always_increment_major: false,
        ..AnalyzerConfig::default()
    };

    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
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

    // In 0.x versions with breaking_always_increment_major=false,
    // breaking changes bump minor instead of major
    assert_eq!(release.tag.unwrap().semver, SemVer::parse("0.2.0").unwrap());
}

#[test]
fn test_custom_major_regex_works_with_breaking_syntax() {
    let config = AnalyzerConfig {
        custom_major_increment_regex: Some("MAJOR".to_string()),
        ..AnalyzerConfig::default()
    };

    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..release::Tag::default()
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
    assert_eq!(release.tag.unwrap().semver, SemVer::parse("1.0.0").unwrap());
}

#[test]
fn test_custom_major_increment_regex() {
    let config = AnalyzerConfig {
        custom_major_increment_regex: Some("doc".to_string()),
        ..AnalyzerConfig::default()
    };

    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..release::Tag::default()
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
    assert_eq!(release.tag.unwrap().semver, SemVer::parse("1.0.0").unwrap());
}

#[test]
fn test_features_always_increment_minor_disabled() {
    let config = AnalyzerConfig {
        features_always_increment_minor: false,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..release::Tag::default()
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
    assert_eq!(release.tag.unwrap().semver, SemVer::parse("0.1.1").unwrap());
}

#[test]
fn test_custom_minor_increment_regex() {
    let config = AnalyzerConfig {
        custom_minor_increment_regex: Some("ci".to_string()),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..release::Tag::default()
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
    assert_eq!(release.tag.unwrap().semver, SemVer::parse("0.2.0").unwrap());
}

#[test]
fn test_custom_minor_regex_works_with_feat_syntax() {
    let config = AnalyzerConfig {
        custom_minor_increment_regex: Some("ci".to_string()),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..release::Tag::default()
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
    assert_eq!(release.tag.unwrap().semver, SemVer::parse("0.2.0").unwrap());
}

#[test]
fn test_both_boolean_flags_disabled_minor_bump() {
    let config = AnalyzerConfig {
        features_always_increment_minor: false,
        breaking_always_increment_major: false,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..release::Tag::default()
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
    assert_eq!(release.tag.unwrap().semver, SemVer::parse("0.2.0").unwrap());
}

#[test]
fn test_both_boolean_flags_disabled_patch_bump() {
    let config = AnalyzerConfig {
        features_always_increment_minor: false,
        breaking_always_increment_major: false,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..release::Tag::default()
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
    assert_eq!(release.tag.unwrap().semver, SemVer::parse("0.1.1").unwrap());
}

#[test]
fn test_custom_regex_matches_non_conventional_commit() {
    let config = AnalyzerConfig {
        custom_major_increment_regex: Some("wow".to_string()),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "0.1.0".to_string(),
        semver: SemVer::parse("0.1.0").unwrap(),
        ..release::Tag::default()
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
    assert_eq!(release.tag.unwrap().semver, SemVer::parse("1.0.0").unwrap());
}
