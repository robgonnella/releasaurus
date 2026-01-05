//! Prerelease versioning tests.
//!
//! Tests for:
//! - Starting prerelease from stable version
//! - Continuing prerelease with same identifier
//! - Graduating prerelease to stable
//! - Switching prerelease identifiers
//! - First release as prerelease
//! - Breaking changes in prerelease
//! - Static vs Versioned prerelease strategies
//! - Prerelease with tag prefix

use crate::{
    analyzer::{Analyzer, config::AnalyzerConfig, release},
    config::prerelease::{PrereleaseConfig, PrereleaseStrategy},
    forge::request::ForgeCommit,
};
use semver::Version as SemVer;

#[test]
fn test_prerelease_start_from_stable() {
    let config = AnalyzerConfig {
        prerelease: Some(PrereleaseConfig {
            suffix: Some("alpha".to_string()),
            strategy: PrereleaseStrategy::Versioned,
        }),
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
        message: "feat: new feature".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    assert_eq!(release.tag.semver, SemVer::parse("1.1.0-alpha.1").unwrap());
}

#[test]
fn test_prerelease_continue_same_identifier() {
    let config = AnalyzerConfig {
        prerelease: Some(PrereleaseConfig {
            suffix: Some("alpha".to_string()),
            strategy: PrereleaseStrategy::Versioned,
        }),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "1.1.0-alpha.1".to_string(),
        semver: SemVer::parse("1.1.0-alpha.1").unwrap(),
        ..release::Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "fix: bug fix".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    assert_eq!(release.tag.semver, SemVer::parse("1.1.0-alpha.2").unwrap());
}

#[test]
fn test_prerelease_graduate_to_stable() {
    let config = AnalyzerConfig {
        prerelease: None,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "1.0.0-alpha.5".to_string(),
        semver: SemVer::parse("1.0.0-alpha.5").unwrap(),
        ..release::Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "fix: final fix".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    assert_eq!(release.tag.semver, SemVer::parse("1.0.0").unwrap());
}

#[test]
fn test_prerelease_switch_identifier() {
    let config = AnalyzerConfig {
        prerelease: Some(PrereleaseConfig {
            suffix: Some("beta".to_string()),
            strategy: PrereleaseStrategy::Versioned,
        }),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "1.0.0-alpha.3".to_string(),
        semver: SemVer::parse("1.0.0-alpha.3").unwrap(),
        ..release::Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat: beta ready".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    // Should switch to beta and calculate next version
    assert_eq!(release.tag.semver, SemVer::parse("1.1.0-beta.1").unwrap());
}

#[test]
fn test_prerelease_first_release() {
    let config = AnalyzerConfig {
        prerelease: Some(PrereleaseConfig {
            suffix: Some("alpha".to_string()),
            strategy: PrereleaseStrategy::Versioned,
        }),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat: initial".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, None).unwrap();

    let release = result.unwrap();
    assert_eq!(release.tag.semver, SemVer::parse("0.1.0-alpha.1").unwrap());
}

#[test]
fn test_prerelease_breaking_change() {
    let config = AnalyzerConfig {
        prerelease: Some(PrereleaseConfig {
            suffix: Some("alpha".to_string()),
            strategy: PrereleaseStrategy::Versioned,
        }),
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
        message: "feat!: breaking change".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    // Breaking change should bump major version
    assert_eq!(release.tag.semver, SemVer::parse("2.0.0-alpha.1").unwrap());
}

#[test]
fn test_new_prerelease_with_static_strategy() {
    let config = AnalyzerConfig {
        prerelease: Some(PrereleaseConfig {
            suffix: Some("dev".to_string()),
            strategy: PrereleaseStrategy::Static,
        }),
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
        message: "feat: new feature".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    // Static strategy should produce version without numeric suffix
    assert_eq!(release.tag.semver, SemVer::parse("1.1.0-dev").unwrap());
}

#[test]
fn test_continuing_prerelease_with_static_strategy() {
    let config = AnalyzerConfig {
        prerelease: Some(PrereleaseConfig {
            suffix: Some("dev".to_string()),
            strategy: PrereleaseStrategy::Static,
        }),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "1.1.0-dev".to_string(),
        semver: SemVer::parse("1.1.0-dev").unwrap(),
        ..release::Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "fix: bug fix".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let result = analyzer.analyze(commits, Some(current_tag)).unwrap();

    let release = result.unwrap();
    // Static strategy increments base version, keeps static suffix
    assert_eq!(release.tag.semver, SemVer::parse("1.1.1-dev").unwrap());
}

#[test]
fn test_prerelease_with_tag_prefix() {
    let config = AnalyzerConfig {
        tag_prefix: Some("v".to_string()),
        prerelease: Some(PrereleaseConfig {
            suffix: Some("rc".to_string()),
            strategy: PrereleaseStrategy::Versioned,
        }),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = release::Tag {
        sha: "old123".to_string(),
        name: "v1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
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
    assert_eq!(release.tag.semver, SemVer::parse("1.1.0-rc.1").unwrap());
    assert_eq!(release.tag.name, "v1.1.0-rc.1");
}
