//! Version type tests.
//!
//! Tests for the non-semantic `VersionType` variants added alongside the
//! version strategy refactor:
//! - `SemanticWithBuild`: semantic version + `{timestamp}.{short_sha}`
//!   build metadata (deterministic, asserted exactly), including
//!   prerelease + build combinations
//! - `SemanticDate`: `year.month.day`
//! - `SemanticDateWithTime`: `year.month.day+hour.minute.second`
//! - `SemanticDateWithTimeMicro`: `year.month.day+hour.minute.second.micro`
//!
//! Date-based strategies derive from `chrono::Utc::now()`, so these tests
//! assert structure (major == current UTC year, build-segment counts)
//! rather than exact values.

use chrono::{Datelike, Timelike, Utc};
use semver::{BuildMetadata, Version as SemVer};

use crate::{
    analyzer::{Analyzer, config::AnalyzerConfig},
    config::{
        prerelease::{PrereleaseConfig, PrereleaseStrategy},
        versioning::VersionType,
    },
    forge::request::{ForgeCommit, Tag, TagVersion},
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
        release.tag.version,
        TagVersion::Semantic(SemVer::parse("0.1.0+1000.abc1234").unwrap())
    );
    assert_eq!(release.tag.version.build(), "1000.abc1234");
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
        version: TagVersion::Semantic(SemVer::parse("1.2.3").unwrap()),
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
    assert_eq!(release.tag.version.to_string(), "1.2.4+2000.def4567");
    assert_eq!(release.tag.version.build(), "2000.def4567");
}

#[test]
fn test_semantic_with_build_versioned_prerelease_first_release() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticWithBuild,
        prerelease: Some(PrereleaseConfig {
            suffix: "alpha".to_string(),
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
        release.tag.version,
        TagVersion::Semantic(
            SemVer::parse("0.1.0-alpha.1+1000.abc1234").unwrap()
        )
    );
    assert_eq!(release.tag.version.pre(), "alpha.1");
    assert_eq!(release.tag.version.build(), "1000.abc1234");
}

#[test]
fn test_semantic_with_build_versioned_prerelease_increment() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticWithBuild,
        prerelease: Some(PrereleaseConfig {
            suffix: "alpha".to_string(),
            strategy: PrereleaseStrategy::Versioned,
        }),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.2.0-alpha.1".to_string(),
        version: TagVersion::Semantic(SemVer::parse("1.2.0-alpha.1").unwrap()),
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
    assert_eq!(release.tag.version.pre(), "alpha.2");
    assert_eq!(release.tag.version.build(), "2000.def4567");
    assert_eq!(
        release.tag.version,
        TagVersion::Semantic(
            SemVer::parse("1.2.0-alpha.2+2000.def4567").unwrap()
        )
    );
}

#[test]
fn test_semantic_with_build_static_prerelease() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticWithBuild,
        prerelease: Some(PrereleaseConfig {
            suffix: "dev".to_string(),
            strategy: PrereleaseStrategy::Static,
        }),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.0.0".to_string(),
        version: TagVersion::Semantic(SemVer::parse("1.0.0").unwrap()),
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
    assert_eq!(release.tag.version.pre(), "dev");
    assert_eq!(release.tag.version.build(), "3000.def4567");
    assert_eq!(
        release.tag.version,
        TagVersion::Semantic(SemVer::parse("1.1.0-dev+3000.def4567").unwrap())
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
        version: TagVersion::Semantic(SemVer::parse("1.0.0-alpha.5").unwrap()),
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

    assert!(release.tag.version.pre().is_empty());
    assert_eq!(release.tag.version.build(), "4000.def4567");
    assert_eq!(
        release.tag.version,
        TagVersion::Semantic(SemVer::parse("1.0.0+4000.def4567").unwrap())
    );
}

/// The tag this strategy wrote last time carries build metadata, so the
/// increment has to start from a clean core: `next_version` and
/// `graduate_prerelease` both copy the current version's metadata forward.
#[test]
fn test_semantic_with_build_replaces_previous_build_metadata() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticWithBuild,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.2.3+2000.def4567".to_string(),
        version: TagVersion::Semantic(
            SemVer::parse("1.2.3+2000.def4567").unwrap(),
        ),
        ..Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc789".to_string(),
        short_id: "abc7890".to_string(),
        message: "fix: another fix".to_string(),
        timestamp: 5000,
        ..ForgeCommit::default()
    }];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    assert_eq!(
        release.tag.version,
        TagVersion::Semantic(SemVer::parse("1.2.4+5000.abc7890").unwrap())
    );
    assert_eq!(release.tag.version.build(), "5000.abc7890");
}

/// Switching from `major.minor.patch+timestamp.sha` back to plain
/// `major.minor.patch` must not inherit the old tag's build metadata, which
/// would otherwise pin a stale timestamp and sha onto every later release.
#[test]
fn test_semantic_clears_build_metadata_from_current_tag() {
    let config = AnalyzerConfig {
        version_type: VersionType::Semantic,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.2.3+2000.def4567".to_string(),
        version: TagVersion::Semantic(
            SemVer::parse("1.2.3+2000.def4567").unwrap(),
        ),
        ..Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc789".to_string(),
        short_id: "abc7890".to_string(),
        message: "fix: another fix".to_string(),
        timestamp: 5000,
        ..ForgeCommit::default()
    }];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    assert_eq!(
        release.tag.version,
        TagVersion::Semantic(SemVer::parse("1.2.4").unwrap())
    );
    assert!(release.tag.version.build().is_empty());
}

/// Same for the graduation path, which clones the current version wholesale
/// and clears only the prerelease identifier.
#[test]
fn test_semantic_clears_build_metadata_when_graduating() {
    let config = AnalyzerConfig {
        version_type: VersionType::Semantic,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: "1.0.0-alpha.5+2000.def4567".to_string(),
        version: TagVersion::Semantic(
            SemVer::parse("1.0.0-alpha.5+2000.def4567").unwrap(),
        ),
        ..Tag::default()
    };

    let commits = vec![ForgeCommit {
        id: "abc789".to_string(),
        short_id: "abc7890".to_string(),
        message: "fix: final fix".to_string(),
        timestamp: 5000,
        ..ForgeCommit::default()
    }];

    let release = analyzer
        .analyze(commits, Some(current_tag))
        .unwrap()
        .unwrap();

    assert_eq!(
        release.tag.version,
        TagVersion::Semantic(SemVer::parse("1.0.0").unwrap())
    );
    assert!(release.tag.version.build().is_empty());
}

#[test]
fn test_date_version() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticDate,
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
    let version = &release.tag.version;
    let now = Utc::now();
    assert_eq!(
        version.to_string(),
        format!("{}.{}.{}", now.year(), now.month(), now.day())
    );
    assert!(version.pre().is_empty());
    assert!(version.build().is_empty());
}

#[test]
fn test_date_with_time_version() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticDateWithTime,
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
    let version = &release.tag.version;
    let now = Utc::now();

    assert_eq!(
        version.to_string(),
        format!(
            "{}.{}.{}+{:02}.{:02}.{:02}",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        )
    );
}

#[test]
fn test_date_with_time_micro_version() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticDateWithTimeMicro,
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
    let version = &release.tag.version;
    let now = Utc::now();

    assert!(version.to_string().starts_with(&format!(
        "{}.{}.{}+{:02}.",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
    )));
}

/// Today's `year.month.day`, the version the date strategy is about to
/// compute.
fn today() -> SemVer {
    let now = Utc::now();
    SemVer::new(now.year() as u64, now.month().into(), now.day().into())
}

fn commit_for_date_test() -> Vec<ForgeCommit> {
    vec![ForgeCommit {
        id: "abc123".to_string(),
        short_id: "abc1234".to_string(),
        message: "feat: new feature".to_string(),
        timestamp: 1000,
        ..ForgeCommit::default()
    }]
}

/// Plain `year.month.day` allows one release per day by design, so a
/// same-day re-run has nothing to release rather than re-tagging the version
/// already out.
#[test]
fn test_date_version_same_day_is_not_releasable() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticDate,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let today = today();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: today.to_string(),
        version: TagVersion::Semantic(today),
        ..Tag::default()
    };

    let release = analyzer
        .analyze(commit_for_date_test(), Some(current_tag))
        .unwrap();

    assert!(release.is_none());
}

/// Date versions come from the clock, not from the current tag, so a tag
/// dated ahead of now (clock skew, or a hand-made tag) must not be answered
/// with a lower version.
#[test]
fn test_date_version_does_not_go_backwards() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticDate,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let future = SemVer::new(Utc::now().year() as u64 + 1, 1, 1);

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: future.to_string(),
        version: TagVersion::Semantic(future),
        ..Tag::default()
    };

    let release = analyzer
        .analyze(commit_for_date_test(), Some(current_tag))
        .unwrap();

    assert!(release.is_none());
}

/// A tag from a previous day still releases today's version.
#[test]
fn test_date_version_advances_from_older_tag() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticDate,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let past = SemVer::new(Utc::now().year() as u64 - 1, 1, 1);

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: past.to_string(),
        version: TagVersion::Semantic(past),
        ..Tag::default()
    };

    let release = analyzer
        .analyze(commit_for_date_test(), Some(current_tag))
        .unwrap()
        .unwrap();

    assert_eq!(release.tag.version.to_string(), today().to_string());
}

/// The time-based variants are free to release repeatedly within one day —
/// the seconds segment advances even when the date segment does not.
#[test]
fn test_date_with_time_version_releases_twice_in_one_day() {
    let config = AnalyzerConfig {
        version_type: VersionType::SemanticDateWithTime,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let mut earlier_today = today();
    earlier_today.build = BuildMetadata::new("00.00.00").unwrap();

    let current_tag = Tag {
        sha: "old123".to_string(),
        name: earlier_today.to_string(),
        version: TagVersion::Semantic(earlier_today),
        ..Tag::default()
    };

    let release = analyzer
        .analyze(commit_for_date_test(), Some(current_tag))
        .unwrap()
        .unwrap();

    let now = Utc::now();

    assert!(release.tag.version.to_string().starts_with(&format!(
        "{}.{}.{}+{:02}.",
        now.year(),
        now.month(),
        now.day(),
        now.hour()
    )));
    assert!(!release.tag.version.build().is_empty());
}

fn strftime_tag(name: &str, fmt: &str) -> Tag {
    Tag {
        sha: "old123".to_string(),
        name: name.to_string(),
        version: TagVersion::parse(name, &VersionType::Strftime(fmt.into()))
            .unwrap(),
        ..Tag::default()
    }
}

#[test]
fn test_strftime_version_renders_current_utc_time_in_format() {
    let config = AnalyzerConfig {
        version_type: VersionType::Strftime("%Y.%m.%d".into()),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let release = analyzer
        .analyze(commit_for_date_test(), None)
        .unwrap()
        .unwrap();

    let rendered = Utc::now().format("%Y.%m.%d").to_string();
    assert_eq!(release.tag.version.to_string(), rendered);
    assert!(
        release.tag.name.ends_with(&rendered),
        "tag name {} should end with {rendered}",
        release.tag.name
    );
    assert!(release.tag.version.pre().is_empty());
    assert!(release.tag.version.build().is_empty());
}

#[test]
fn test_strftime_version_same_period_is_not_releasable() {
    let config = AnalyzerConfig {
        version_type: VersionType::Strftime("%Y.%m.%d".into()),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let today = Utc::now().format("%Y.%m.%d").to_string();
    let current_tag = strftime_tag(&today, "%Y.%m.%d");

    let result = analyzer
        .analyze(commit_for_date_test(), Some(current_tag))
        .unwrap();
    assert!(result.is_none(), "same-day re-run should not release");
}

#[test]
fn test_strftime_version_does_not_go_backwards() {
    let config = AnalyzerConfig {
        version_type: VersionType::Strftime("%Y.%m.%d".into()),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = strftime_tag("2999.01.01", "%Y.%m.%d");

    let result = analyzer
        .analyze(commit_for_date_test(), Some(current_tag))
        .unwrap();
    assert!(
        result.is_none(),
        "a future-dated tag must not be superseded"
    );
}

#[test]
fn test_strftime_version_advances_from_older_tag() {
    let config = AnalyzerConfig {
        version_type: VersionType::Strftime("%Y.%m.%d".into()),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let current_tag = strftime_tag("2020.01.01", "%Y.%m.%d");

    let release = analyzer
        .analyze(commit_for_date_test(), Some(current_tag))
        .unwrap()
        .unwrap();

    assert_eq!(
        release.tag.version.to_string(),
        Utc::now().format("%Y.%m.%d").to_string()
    );
}

#[test]
fn test_strftime_version_with_time_releases_twice_in_one_day() {
    let fmt = "%Y.%m.%d.%H%M%S";
    let config = AnalyzerConfig {
        version_type: VersionType::Strftime(fmt.into()),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let an_hour_ago = (Utc::now() - chrono::Duration::hours(1))
        .format(fmt)
        .to_string();
    let current_tag = strftime_tag(&an_hour_ago, fmt);

    let release = analyzer
        .analyze(commit_for_date_test(), Some(current_tag))
        .unwrap()
        .unwrap();

    assert!(
        release.tag.version
            > TagVersion::parse(
                &an_hour_ago,
                &VersionType::Strftime(fmt.into())
            )
            .unwrap()
    );
    assert_eq!(release.tag.version.to_string().len(), an_hour_ago.len());
}

#[test]
fn test_strftime_version_ignores_prerelease_config() {
    let config = AnalyzerConfig {
        version_type: VersionType::Strftime("%Y.%m.%d".into()),
        prerelease: Some(PrereleaseConfig {
            suffix: "alpha".into(),
            strategy: PrereleaseStrategy::Versioned,
        }),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let release = analyzer
        .analyze(commit_for_date_test(), None)
        .unwrap()
        .unwrap();

    assert_eq!(
        release.tag.version.to_string(),
        Utc::now().format("%Y.%m.%d").to_string()
    );
}
