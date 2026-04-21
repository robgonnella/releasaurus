//! Tests for package preparation functionality.
//!
//! Tests for:
//! - Generating prepared packages with dummy commits
//! - Skipping untagged packages
//! - Filtering by target packages
//! - Aggregating prerelease changelogs when graduating to stable

use crate::{
    config::{
        Config,
        changelog::ChangelogConfig,
        package::{PackageConfig, PackageConfigBuilder},
    },
    forge::{
        request::{ForgeCommit, ForgeCommitBuilder, Tag},
        traits::MockForge,
    },
};

use super::common::*;

#[tokio::test]
async fn generate_prepared_with_dummy_commit_skips_untagged_packages() {
    let mut mock_forge = MockForge::new();

    mock_forge
        .expect_get_latest_tags_for_prefix()
        .returning(|_, _| Ok(vec![])); // No tags exist

    let orchestrator = create_core(mock_forge, None, None);

    let prepared = orchestrator
        .generate_prepared_with_dummy_commit(None)
        .await
        .unwrap();
    // Should skip untagged package
    assert_eq!(prepared.len(), 0);
}

#[tokio::test]
async fn generate_prepared_with_dummy_commit_filters_by_targets() {
    let pkg_configs = vec![
        PackageConfigBuilder::default()
            .name("pkg-a")
            .path("packages/pkg-a")
            .build()
            .unwrap(),
        PackageConfigBuilder::default()
            .name("pkg-b")
            .path("packages/pkg-b")
            .build()
            .unwrap(),
    ];

    let mut mock_forge = MockForge::new();

    mock_forge.expect_get_latest_tags_for_prefix().returning(
        |prefix, _branch| {
            Ok(vec![Tag {
                name: format!("{prefix}1.0.0"),
                timestamp: Some(1000),
                ..Default::default()
            }])
        },
    );

    let orchestrator = create_core(mock_forge, Some(pkg_configs), None);

    let prepared = orchestrator
        .generate_prepared_with_dummy_commit(Some(vec!["pkg-a".to_string()]))
        .await
        .unwrap();
    // Should only include pkg-a
    assert_eq!(prepared.len(), 1);
    assert_eq!(prepared[0].name, "pkg-a");
}

// --- aggregate_prereleases tests ---

// Shared test data for graduation scenario:
// - prerelease tag at timestamp 1000 (current version)
// - stable tag at timestamp 0 (last stable release)
// - historical commit at timestamp 500 (between stable and prerelease)
// - current commit at timestamp 2000 (after prerelease)

fn prerelease_tag() -> Tag {
    Tag {
        name: "v1.0.0-rc.1".to_string(),
        semver: semver::Version::parse("1.0.0-rc.1").unwrap(),
        sha: "sha-rc1".to_string(),
        timestamp: Some(1000),
    }
}

fn stable_tag() -> Tag {
    Tag {
        name: "v0.9.0".to_string(),
        semver: semver::Version::parse("0.9.0").unwrap(),
        sha: "sha-0.9.0".to_string(),
        timestamp: Some(0),
    }
}

fn pkg_commit(id: &str, ts: i64) -> ForgeCommit {
    ForgeCommitBuilder::default()
        .id(id)
        .short_id(id)
        .message(format!("feat: {id}"))
        .timestamp(ts)
        .files(vec!["packages/pkg-a/src/lib.rs".to_string()])
        .build()
        .unwrap()
}

fn aggregate_config() -> Config {
    Config {
        changelog: ChangelogConfig {
            aggregate_prereleases: true,
            ..ChangelogConfig::default()
        },
        ..Config::default()
    }
}

fn graduating_pkg() -> PackageConfig {
    PackageConfigBuilder::default()
        .name("test-pkg")
        .path("packages/pkg-a")
        .build()
        .unwrap()
}

#[tokio::test]
async fn aggregate_prereleases_disabled_skips_extra_fetch() {
    // aggregate_prereleases = false (default): even though the package is
    // graduating to stable, no extra tag or commit fetch should happen.
    let mut mock = MockForge::new();
    let pre_tag = prerelease_tag();

    mock.expect_get_latest_tags_for_prefix()
        .times(1)
        .returning(move |_, _| Ok(vec![pre_tag.clone()]));
    mock.expect_get_commits()
        .times(1)
        .returning(|_, _| Ok(vec![]));

    let core = create_core(mock, Some(vec![graduating_pkg()]), None);

    let prepared = core.prepare_packages(None).await.unwrap();
    assert_eq!(prepared.len(), 1);
}

#[tokio::test]
async fn aggregate_prereleases_enabled_not_graduating_skips_extra_fetch() {
    // aggregate_prereleases = true but the current tag is already stable —
    // no graduation is occurring so no extra fetch should happen.
    let mut mock = MockForge::new();
    let s_tag = stable_tag();

    mock.expect_get_latest_tags_for_prefix()
        .times(1)
        .returning(move |_, _| Ok(vec![s_tag.clone()]));
    mock.expect_get_commits()
        .times(1)
        .returning(|_, _| Ok(vec![]));

    let core = create_core(
        mock,
        Some(vec![graduating_pkg()]),
        Some(aggregate_config()),
    );

    let prepared = core.prepare_packages(None).await.unwrap();
    assert_eq!(prepared.len(), 1);
}

#[tokio::test]
async fn aggregate_prereleases_enabled_and_graduating_merges_commits() {
    // Graduating from prerelease to stable with aggregate_prereleases = true.
    // Commits from the prior prerelease cycle should be merged into the
    // prepared package's commit list.
    let mut mock = MockForge::new();
    let mut seq = mockall::Sequence::new();

    let pre_tag = prerelease_tag();
    let s_tag = stable_tag();
    let current = pkg_commit("current", 2000);
    let historical = pkg_commit("historical", 500);
    let current_c = current.clone();
    let historical_c = historical.clone();

    // Step 1: collect current tag for the package
    mock.expect_get_latest_tags_for_prefix()
        .once()
        .in_sequence(&mut seq)
        .returning(move |_, _| Ok(vec![pre_tag.clone()]));

    // Step 2: fetch commits since the prerelease tag SHA
    mock.expect_get_commits()
        .once()
        .in_sequence(&mut seq)
        .returning(move |_, _| Ok(vec![current_c.clone()]));

    // Step 3: look up last stable tag for aggregation
    mock.expect_get_latest_tags_for_prefix()
        .once()
        .in_sequence(&mut seq)
        .returning(move |_, _| Ok(vec![s_tag.clone()]));

    // Step 4: fetch commits since the stable tag SHA (historical range)
    mock.expect_get_commits()
        .once()
        .in_sequence(&mut seq)
        .returning(move |_, _| Ok(vec![historical_c.clone(), current.clone()]));

    let core = create_core(
        mock,
        Some(vec![graduating_pkg()]),
        Some(aggregate_config()),
    );

    let prepared = core.prepare_packages(None).await.unwrap();
    assert_eq!(prepared.len(), 1);
    // historical (500) and current (2000); current already in window
    assert_eq!(prepared[0].commits.len(), 2);
    // sorted by timestamp: historical first
    assert_eq!(prepared[0].commits[0].id, "historical");
    assert_eq!(prepared[0].commits[1].id, "current");
}

#[tokio::test]
async fn aggregate_prereleases_deduplicates_overlapping_commits() {
    // The historical fetch may return commits already present in the
    // current-window fetch. Each commit must appear only once.
    let mut mock = MockForge::new();
    let mut seq = mockall::Sequence::new();

    let pre_tag = prerelease_tag();
    let s_tag = stable_tag();
    let current = pkg_commit("current", 2000);
    let current_c = current.clone();
    let current_c2 = current.clone();

    mock.expect_get_latest_tags_for_prefix()
        .once()
        .in_sequence(&mut seq)
        .returning(move |_, _| Ok(vec![pre_tag.clone()]));

    mock.expect_get_commits()
        .once()
        .in_sequence(&mut seq)
        .returning(move |_, _| Ok(vec![current_c.clone()]));

    mock.expect_get_latest_tags_for_prefix()
        .once()
        .in_sequence(&mut seq)
        .returning(move |_, _| Ok(vec![s_tag.clone()]));

    // Historical window contains ONLY the same commit as current window.
    mock.expect_get_commits()
        .once()
        .in_sequence(&mut seq)
        .returning(move |_, _| Ok(vec![current_c2.clone()]));

    let core = create_core(
        mock,
        Some(vec![graduating_pkg()]),
        Some(aggregate_config()),
    );

    let prepared = core.prepare_packages(None).await.unwrap();
    assert_eq!(prepared.len(), 1);
    assert_eq!(
        prepared[0].commits.len(),
        1,
        "duplicate commit should appear only once"
    );
    assert_eq!(prepared[0].commits[0].id, "current");
}
