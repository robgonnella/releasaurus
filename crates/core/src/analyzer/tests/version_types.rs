//! Version type tests.
//!
//! Tests for the non-semantic `VersionType` variants added alongside the
//! version strategy refactor:
//! - `SemanticWithBuild`: semantic version + `{timestamp}.{short_sha}`
//!   build metadata (deterministic, asserted exactly), including
//!   prerelease + build combinations
//! - `Date`: `year.month.day`
//! - `DateWithTime`: `year.month.day+hour.minute.second`
//! - `DateWithTimeMicro`: `year.month.day+hour.minute.second.micro`
//!
//! Date-based strategies derive from `chrono::Utc::now()`, so these tests
//! assert structure (major == current UTC year, build-segment counts)
//! rather than exact values.

use chrono::{Datelike, Utc};
use semver::Version as SemVer;

use crate::{
    analyzer::{Analyzer, config::AnalyzerConfig},
    config::{
        VersionType,
        prerelease::{PrereleaseConfig, PrereleaseStrategy},
    },
    forge::request::{ForgeCommit, Tag},
};

#[test]
fn test_semantic_with_build_first_release() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticWithBuild,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        short_id: "abc1234".to_string(),
        message: "feat: initial".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    // First release starts at 0.1.0 with build metadata appended.
    assert_eq!(
        release.tag.semver,
        SemVer::parse("0.1.0+1000.abc1234").unwrap()
    );
    assert_eq!(release.tag.semver.build.as_str(), "1000.abc1234");
}

#[test]
fn test_semantic_with_build_increment_from_stable() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticWithBuild,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.2.3".to_string(),
        semver: SemVer::parse("1.2.3").unwrap(),
        ..Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "def456".to_string(),
        short_id: "def4567".to_string(),
        message: "fix: bug fix".to_string(),
        timestamp: 2000,
        ..ForgeCommit::default()
    }];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    // Base bumps patch; build metadata reflects newest commit.
    assert_eq!(release.tag.semver.major, 1);
    assert_eq!(release.tag.semver.minor, 2);
    assert_eq!(release.tag.semver.patch, 4);
    assert_eq!(release.tag.semver.build.as_str(), "2000.def4567");
}

#[test]
fn test_semantic_with_build_versioned_prerelease_first_release() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticWithBuild,
        prerelease: Some(PrereleaseConfig {
            suffix: Some("alpha".to_string()),
            strategy: PrereleaseStrategy::Versioned,
        }),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        short_id: "abc1234".to_string(),
        message: "feat: initial".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();

    // First release carries both a versioned prerelease and build metadata.
    assert_eq!(
        release.tag.semver,
        SemVer::parse("0.1.0-alpha.1+1000.abc1234").unwrap()
    );
    assert_eq!(release.tag.semver.pre.as_str(), "alpha.1");
    assert_eq!(release.tag.semver.build.as_str(), "1000.abc1234");
}

#[test]
fn test_semantic_with_build_versioned_prerelease_increment() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticWithBuild,
        prerelease: Some(PrereleaseConfig {
            suffix: Some("alpha".to_string()),
            strategy: PrereleaseStrategy::Versioned,
        }),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.2.0-alpha.1".to_string(),
        semver: SemVer::parse("1.2.0-alpha.1").unwrap(),
        ..Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "def456".to_string(),
        short_id: "def4567".to_string(),
        message: "fix: bug fix".to_string(),
        timestamp: 2000,
        ..ForgeCommit::default()
    }];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    // Existing prerelease increments rather than graduating; build refreshed.
    assert_eq!(release.tag.semver.pre.as_str(), "alpha.2");
    assert_eq!(release.tag.semver.build.as_str(), "2000.def4567");
    assert_eq!(
        release.tag.semver,
        SemVer::parse("1.2.0-alpha.2+2000.def4567").unwrap()
    );
}

#[test]
fn test_semantic_with_build_static_prerelease() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticWithBuild,
        prerelease: Some(PrereleaseConfig {
            suffix: Some("dev".to_string()),
            strategy: PrereleaseStrategy::Static,
        }),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "def456".to_string(),
        short_id: "def4567".to_string(),
        message: "feat: new feature".to_string(),
        timestamp: 3000,
        ..ForgeCommit::default()
    }];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    // Static suffix has no numeric counter; build metadata appended.
    assert_eq!(release.tag.semver.pre.as_str(), "dev");
    assert_eq!(release.tag.semver.build.as_str(), "3000.def4567");
    assert_eq!(
        release.tag.semver,
        SemVer::parse("1.1.0-dev+3000.def4567").unwrap()
    );
}

#[test]
fn test_semantic_with_build_graduate_prerelease() {
    // No prerelease config: a current prerelease tag graduates to stable,
    // still carrying fresh build metadata.
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticWithBuild,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.0.0-alpha.5".to_string(),
        semver: SemVer::parse("1.0.0-alpha.5").unwrap(),
        ..Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "def456".to_string(),
        short_id: "def4567".to_string(),
        message: "fix: final fix".to_string(),
        timestamp: 4000,
        ..ForgeCommit::default()
    }];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    assert!(release.tag.semver.pre.is_empty());
    assert_eq!(release.tag.semver.build.as_str(), "4000.def4567");
    assert_eq!(
        release.tag.semver,
        SemVer::parse("1.0.0+4000.def4567").unwrap()
    );
}

#[test]
fn test_date_version() {
    let config = AnalyzerConfig {
        version_type: VersionType::Date,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    // Date strategy ignores commits/tag; a non-empty commit list still
    // yields a date-shaped version.
    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat: new feature".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();
    let version = &release.tag.semver;

    assert_eq!(version.major, Utc::now().year() as u64);
    assert!((1..=12).contains(&version.minor));
    assert!((1..=31).contains(&version.patch));
    assert!(version.pre.is_empty());
    assert!(version.build.is_empty());
}

#[test]
fn test_date_with_time_version() {
    let config = AnalyzerConfig {
        version_type: VersionType::DateWithTime,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat: new feature".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();
    let version = &release.tag.semver;

    assert_eq!(version.major, Utc::now().year() as u64);

    // Build metadata is hour.minute.second — three numeric segments.
    let segments: Vec<&str> = version.build.as_str().split('.').collect();
    assert_eq!(segments.len(), 3);
    assert!(segments.iter().all(|s| s.parse::<u64>().is_ok()));
}

#[test]
fn test_date_with_time_micro_version() {
    let config = AnalyzerConfig {
        version_type: VersionType::DateWithTimeMicro,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![ForgeCommit {
        id: "abc123".to_string(),
        message: "feat: new feature".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }];

    let release = analyzer.analyze(commits, None).unwrap().unwrap();
    let version = &release.tag.semver;

    assert_eq!(version.major, Utc::now().year() as u64);

    // Build metadata is hour.minute.second.micro — four numeric segments.
    let segments: Vec<&str> = version.build.as_str().split('.').collect();
    assert_eq!(segments.len(), 4);
    assert!(segments.iter().all(|s| s.parse::<u64>().is_ok()));
}
