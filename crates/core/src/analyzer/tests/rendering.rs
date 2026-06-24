//! Default `body` template rendering tests.
//!
//! The other analyzer test modules use `AnalyzerConfig::default()`, whose
//! `body` is empty, so they never exercise the template itself. These tests
//! render [`DEFAULT_BODY`] to pin the group-ordering mechanism:
//! [`Parser::order`] is synthesized into an `<!-- NN -->` prefix on
//! `Commit::group`, the template sorts on it, and `striptags` removes it
//! before the heading is written.

use regex::Regex;
use semver::Version as SemVer;

use crate::{
    analyzer::{Analyzer, config::AnalyzerConfig},
    config::{
        changelog::DEFAULT_BODY,
        versioning::{Group, NAMED_PARSERS, Parser},
    },
    forge::request::{ForgeCommit, Tag},
};

fn make_commit(id: &str, message: &str, timestamp: i64) -> ForgeCommit {
    ForgeCommit {
        id: id.to_string(),
        short_id: id.to_string(),
        message: message.to_string(),
        timestamp,
        ..ForgeCommit::default()
    }
}

fn current_tag() -> Tag {
    Tag {
        sha: "old123".to_string(),
        name: "v1.0.0".to_string(),
        semver: SemVer::parse("1.0.0").unwrap(),
        ..Tag::default()
    }
}

/// Returns the `### ` headings from rendered notes, in render order.
fn headings(notes: &str) -> Vec<String> {
    notes
        .lines()
        .filter_map(|l| l.strip_prefix("### "))
        .map(|l| l.to_string())
        .collect()
}

#[test]
fn default_body_strips_order_tags_from_headings() {
    // Order is carried as a `<!-- NN -->` prefix on `Commit::group`; the
    // template must strip it so only the title reaches the heading.
    let config = AnalyzerConfig {
        body: DEFAULT_BODY.into(),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        make_commit("aaa1111", "fix: a bug fix", 1000),
        make_commit("bbb2222", "feat: a feature", 2000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag()))
        .unwrap()
        .unwrap();

    // The tag is an ordering device only - it must never reach the changelog.
    assert!(
        !release.notes.contains("<!--"),
        "order tag leaked into notes:\n{}",
        release.notes
    );

    let feature_title = NAMED_PARSERS[&Group::Feature].title.clone().unwrap();
    let fix_title = NAMED_PARSERS[&Group::Fix].title.clone().unwrap();

    assert_eq!(
        headings(&release.notes),
        vec![
            feature_title.replace("<!-- 01 -->", ""),
            fix_title.replace("<!-- 02 -->", ""),
        ]
    );
}

#[test]
fn default_body_orders_groups_by_order_not_alphabetically() {
    // "Features" sorts after "Bug Fixes" alphabetically, so a run that gets
    // the ordering right can only be doing so via `Parser::order`.
    let config = AnalyzerConfig {
        body: DEFAULT_BODY.into(),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    // Deliberately feed the fix first.
    let commits = vec![
        make_commit("aaa1111", "fix: a bug fix", 1000),
        make_commit("bbb2222", "feat: a feature", 2000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag()))
        .unwrap()
        .unwrap();

    let rendered = headings(&release.notes);
    let features = rendered.iter().position(|h| h.contains("Features"));
    let fixes = rendered.iter().position(|h| h.contains("Bug Fixes"));

    assert!(
        features < fixes,
        "expected Features before Bug Fixes, got {rendered:?}"
    );
}

#[test]
fn default_body_places_custom_group_by_its_order() {
    // A custom group with order 3 must land between Bug Fixes (2) and
    // Chore (9), and render without the synthesized sort tag.
    let config = AnalyzerConfig {
        body: DEFAULT_BODY.into(),
        custom_parsers: vec![Parser::new(
            Some(Regex::new(r"^deps").unwrap()),
            "📦 Dependencies".into(),
            false,
            3,
        )],
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        make_commit("aaa1111", "chore: tidy up", 1000),
        make_commit("bbb2222", "deps: bump serde", 2000),
        make_commit("ccc3333", "fix: a bug fix", 3000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag()))
        .unwrap()
        .unwrap();

    let rendered = headings(&release.notes);

    assert_eq!(rendered.len(), 3, "unexpected headings: {rendered:?}");
    assert!(rendered[0].contains("Bug Fixes"), "got {rendered:?}");
    assert_eq!(rendered[1], "📦 Dependencies");
    assert!(rendered[2].contains("Chore"), "got {rendered:?}");
}

/// Retitling a group must not move it.
///
/// Order used to be the byte order of the title itself, so a title that
/// didn't happen to start with an `<!-- NN -->` tag sorted after every one
/// that did - silently dropping the retitled group to the bottom of the
/// changelog. `order` is now independent of the text.
#[test]
fn default_body_keeps_group_position_when_retitled() {
    let mut named_parsers = NAMED_PARSERS.clone();
    named_parsers.get_mut(&Group::Feature).unwrap().title =
        Some("✨ New Stuff".into());

    let config = AnalyzerConfig {
        body: DEFAULT_BODY.into(),
        named_parsers,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        make_commit("aaa1111", "chore: tidy up", 1000),
        make_commit("bbb2222", "feat: a feature", 2000),
        make_commit("ccc3333", "fix: a bug fix", 3000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag()))
        .unwrap()
        .unwrap();

    let rendered = headings(&release.notes);

    // Features keeps order 1, so it stays first despite the new text.
    assert_eq!(rendered.len(), 3, "unexpected headings: {rendered:?}");
    assert_eq!(rendered[0], "✨ New Stuff");
    assert!(rendered[1].contains("Bug Fixes"), "got {rendered:?}");
    assert!(rendered[2].contains("Chore"), "got {rendered:?}");
}

#[test]
fn default_body_marks_breaking_commits() {
    let config = AnalyzerConfig {
        body: DEFAULT_BODY.into(),
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![make_commit(
        "aaa1111",
        "feat!: drop legacy api\n\nBREAKING CHANGE: the v1 api is gone",
        1000,
    )];

    let release = analyzer
        .analyze(commits, Some(current_tag()))
        .unwrap()
        .unwrap();

    assert_eq!(headings(&release.notes), vec!["❌ Breaking".to_string()]);
    assert!(
        release.notes.contains("[**breaking**]: drop legacy api"),
        "missing breaking marker:\n{}",
        release.notes
    );
    assert!(
        release.notes.contains("> the v1 api is gone"),
        "missing breaking description:\n{}",
        release.notes
    );
}

#[test]
fn default_body_omits_merge_commits_from_rendered_notes() {
    // `skip_merge_commits = false` keeps merge commits in `release.commits`,
    // but the default template filters them out of the rendered notes.
    let config = AnalyzerConfig {
        body: DEFAULT_BODY.into(),
        skip_merge_commits: false,
        ..AnalyzerConfig::default()
    };
    let analyzer = Analyzer::new(&config).unwrap();

    let commits = vec![
        ForgeCommit {
            merge_commit: true,
            ..make_commit("aaa1111", "Merge pull request #1", 1000)
        },
        make_commit("bbb2222", "fix: a bug fix", 2000),
    ];

    let release = analyzer
        .analyze(commits, Some(current_tag()))
        .unwrap()
        .unwrap();

    assert_eq!(release.commits.len(), 2);
    assert!(
        !release.notes.contains("Merge pull request"),
        "merge commit leaked into notes:\n{}",
        release.notes
    );
}
