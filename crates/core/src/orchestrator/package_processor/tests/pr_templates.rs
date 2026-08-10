//! Tests for the Tera templates that generate release commit messages
//! and PR titles.
//!
//! Two tiers are in play, selected by config alone: a per-package
//! template when `separate_pull_requests` is on or a single package is
//! configured, and a `monorepo_*` template otherwise. What is actually
//! being released does not enter into it, which is what keeps the format
//! stable across runs — see `render_release_template`.

use std::sync::{Arc, Mutex};

use super::common::*;

use crate::{
    config::{
        Config,
        defaults::DefaultsConfig,
        package::{PackageConfig, PackageConfigBuilder},
        repository::RepositoryConfig,
    },
    forge::traits::MockForge,
    result::ReleasaurusError,
};

/// Wires up the mock calls every path through `create_pr_branches` makes,
/// and captures the commit messages handed to the forge.
fn prepare_mock(branches: usize) -> (MockForge, Arc<Mutex<Vec<String>>>) {
    let mut mock = MockForge::new();

    mock.expect_get_open_release_pr().returning(|_| Ok(None));
    mock.expect_get_file_content().returning(|_| Ok(None));
    expect_html_comment_encoding(&mut mock);

    let messages = capture_release_branch_messages(&mut mock, branches);

    (mock, messages)
}

/// Config carrying only template overrides.
fn config(
    separate_pull_requests: bool,
    defaults: DefaultsConfig,
) -> Option<Config> {
    Some(Config {
        repository: RepositoryConfig {
            separate_pull_requests,
            ..RepositoryConfig::default()
        },
        defaults,
        ..Config::default()
    })
}

fn package_defaults(commit_message: &str, pr_title: &str) -> DefaultsConfig {
    DefaultsConfig {
        commit_message_template: Some(commit_message.into()),
        pr_title_template: Some(pr_title.into()),
        ..DefaultsConfig::default()
    }
}

fn monorepo_defaults(commit_message: &str, pr_title: &str) -> DefaultsConfig {
    DefaultsConfig {
        monorepo_commit_message_template: Some(commit_message.into()),
        monorepo_pr_title_template: Some(pr_title.into()),
        ..DefaultsConfig::default()
    }
}

fn two_packages() -> Vec<PackageConfig> {
    vec![
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
    ]
}

/// A repo with one configured package can only ever produce a
/// single-package PR, so it uses the per-package template regardless of
/// `separate_pull_requests`. The monorepo templates are set here too, so
/// the test fails if the wrong tier is picked rather than silently
/// falling back to a built-in default.
#[tokio::test]
async fn single_configured_package_uses_package_template() {
    let (mock, messages) = prepare_mock(1);

    let processor = create_package_processor(
        mock,
        None,
        config(
            false,
            DefaultsConfig {
                commit_message_template: Some("pkg: {{ package_name }}".into()),
                pr_title_template: Some("pkg title: {{ tag }}".into()),
                monorepo_commit_message_template: Some(
                    "mono: {{ repo_name }}".into(),
                ),
                monorepo_pr_title_template: Some(
                    "mono title: {{ repo_name }}".into(),
                ),
                ..DefaultsConfig::default()
            },
        ),
    );

    let groups = processor
        .group_releasable_packages(vec![releasable("test-pkg", "v1.2.3")]);

    let grouped = processor.release_pr_bundles(groups).await.unwrap();

    let requests = processor.create_pr_branches(grouped).await.unwrap();

    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].request.title, "pkg title: v1.2.3");
    assert_eq!(messages.lock().unwrap().as_slice(), ["pkg: test-pkg"]);
}

/// Multiple configured packages with `separate_pull_requests = false`
/// means a combined PR, which uses the monorepo templates.
#[tokio::test]
async fn multiple_configured_packages_use_monorepo_template() {
    let (mock, messages) = prepare_mock(1);

    let processor = create_package_processor(
        mock,
        Some(two_packages()),
        config(
            false,
            monorepo_defaults(
                "mono commit: {{ repo_name }}",
                "mono title: {{ repo_name }} on {{ branch }}",
            ),
        ),
    );

    let groups = processor.group_releasable_packages(vec![
        releasable("pkg-a", "v1.0.0"),
        releasable("pkg-b", "v2.0.0"),
    ]);

    let grouped = processor.release_pr_bundles(groups).await.unwrap();

    let requests = processor.create_pr_branches(grouped).await.unwrap();

    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].request.title, "mono title: test-repo on main");
    assert_eq!(
        messages.lock().unwrap().as_slice(),
        ["mono commit: test-repo"]
    );
}

/// The selection depends only on config, so a combined-PR monorepo keeps
/// using the monorepo templates on runs where a single package has
/// changes.
#[tokio::test]
async fn combined_pr_uses_monorepo_template_when_one_package_releases() {
    let (mock, messages) = prepare_mock(1);

    let pkg_configs = vec![
        PackageConfigBuilder::default()
            .name("pkg-a")
            .path("packages/pkg-a")
            .commit_message_template("A commit {{ tag }}")
            .pr_title_template("A title {{ tag }}")
            .build()
            .unwrap(),
        PackageConfigBuilder::default()
            .name("pkg-b")
            .path("packages/pkg-b")
            .build()
            .unwrap(),
    ];

    let processor = create_package_processor(
        mock,
        Some(pkg_configs),
        config(
            false,
            monorepo_defaults(
                "mono commit: {{ repo_name }}",
                "mono title: {{ repo_name }}",
            ),
        ),
    );

    // Only pkg-a has anything to release this run.
    let groups = processor
        .group_releasable_packages(vec![releasable("pkg-a", "v1.0.0")]);

    let grouped = processor.release_pr_bundles(groups).await.unwrap();

    let requests = processor.create_pr_branches(grouped).await.unwrap();

    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].request.title, "mono title: test-repo");
    assert_eq!(
        messages.lock().unwrap().as_slice(),
        ["mono commit: test-repo"]
    );
}

/// Each package carries its own resolved templates through to the PR it
/// ends up in, rather than the first package's winning for all of them.
#[tokio::test]
async fn separate_prs_use_each_packages_own_template() {
    let (mock, messages) = prepare_mock(2);

    let pkg_configs = vec![
        PackageConfigBuilder::default()
            .name("pkg-a")
            .path("packages/pkg-a")
            .commit_message_template("A commit {{ tag }}")
            .pr_title_template("A title {{ tag }}")
            .build()
            .unwrap(),
        PackageConfigBuilder::default()
            .name("pkg-b")
            .path("packages/pkg-b")
            .commit_message_template("B commit {{ tag }}")
            .pr_title_template("B title {{ tag }}")
            .build()
            .unwrap(),
    ];

    let processor = create_package_processor(
        mock,
        Some(pkg_configs),
        config(true, DefaultsConfig::default()),
    );

    let groups = processor.group_releasable_packages(vec![
        releasable("pkg-a", "v1.0.0"),
        releasable("pkg-b", "v2.0.0"),
    ]);

    let grouped = processor.release_pr_bundles(groups).await.unwrap();

    let requests = processor.create_pr_branches(grouped).await.unwrap();

    assert_eq!(requests.len(), 2);

    let mut titles: Vec<&str> =
        requests.iter().map(|r| r.request.title.as_str()).collect();
    titles.sort();
    assert_eq!(titles, ["A title v1.0.0", "B title v2.0.0"]);

    let mut commits = messages.lock().unwrap().clone();
    commits.sort();
    assert_eq!(commits, ["A commit v1.0.0", "B commit v2.0.0"]);
}

/// Pins the documented per-package context, including that `tag` carries
/// the tag prefix while `semver` is the bare version.
#[tokio::test]
async fn renders_all_package_context_variables() {
    let (mock, messages) = prepare_mock(1);

    let all = "{{ branch }}|{{ repo_name }}|{{ package_name }}|{{ tag }}|\
               {{ semver }}";

    let processor = create_package_processor(
        mock,
        None,
        config(true, package_defaults(all, all)),
    );

    let groups = processor
        .group_releasable_packages(vec![releasable("test-pkg", "v1.2.3")]);

    let grouped = processor.release_pr_bundles(groups).await.unwrap();

    let requests = processor.create_pr_branches(grouped).await.unwrap();

    let expected = "main|test-repo|test-pkg|v1.2.3|1.2.3";

    assert_eq!(requests[0].request.title, expected);
    assert_eq!(messages.lock().unwrap()[0], expected);
}

/// Pins the documented monorepo context, which is deliberately narrower.
#[tokio::test]
async fn renders_all_monorepo_context_variables() {
    let (mock, messages) = prepare_mock(1);

    let all = "{{ branch }}|{{ repo_name }}";

    let processor = create_package_processor(
        mock,
        Some(two_packages()),
        config(false, monorepo_defaults(all, all)),
    );

    let groups = processor.group_releasable_packages(vec![
        releasable("pkg-a", "v1.0.0"),
        releasable("pkg-b", "v2.0.0"),
    ]);

    let grouped = processor.release_pr_bundles(groups).await.unwrap();

    let requests = processor.create_pr_branches(grouped).await.unwrap();

    assert_eq!(requests[0].request.title, "main|test-repo");
    assert_eq!(messages.lock().unwrap()[0], "main|test-repo");
}

/// The two built-in defaults, rendered. These strings are what the book
/// documents, and changing either is a user-visible change.
#[tokio::test]
async fn defaults_produce_the_documented_commit_and_title() {
    let (mock, messages) = prepare_mock(1);

    let processor = create_package_processor(mock, None, None);

    let groups = processor
        .group_releasable_packages(vec![releasable("test-pkg", "v1.2.3")]);

    let grouped = processor.release_pr_bundles(groups).await.unwrap();

    let requests = processor.create_pr_branches(grouped).await.unwrap();

    assert_eq!(
        requests[0].request.title,
        "chore(main): release test-pkg v1.2.3"
    );
    assert_eq!(
        messages.lock().unwrap()[0],
        "chore(main): release test-pkg v1.2.3"
    );

    // ... and the monorepo default, once a combined PR holds two packages
    let (mock, messages) = prepare_mock(1);

    let processor = create_package_processor(
        mock,
        Some(two_packages()),
        config(false, DefaultsConfig::default()),
    );

    let groups = processor.group_releasable_packages(vec![
        releasable("pkg-a", "v1.0.0"),
        releasable("pkg-b", "v2.0.0"),
    ]);

    let grouped = processor.release_pr_bundles(groups).await.unwrap();

    let requests = processor.create_pr_branches(grouped).await.unwrap();

    assert_eq!(requests[0].request.title, "chore(main): release test-repo");
    assert_eq!(
        messages.lock().unwrap()[0],
        "chore(main): release test-repo"
    );
}

/// Templates now come from resolved config rather than travelling on the
/// package, and resolution render-tests every one against probe values.
/// That leaves a name with no config entry as the reachable failure, and
/// it must surface as an error rather than a malformed commit message.
#[test]
fn render_failure_surfaces_a_missing_package_config() {
    let processor = create_package_processor(MockForge::new(), None, None);

    let pkgs = vec![releasable("not-configured", "v1.2.3")];

    let err = processor
        .release_commit_message_for_pr_package_list(&pkgs)
        .unwrap_err();

    assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));

    let err = processor
        .release_pr_title_for_pr_package_list(&pkgs)
        .unwrap_err();

    assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));
}

/// Rendering reads `packages[0]`, so an empty bundle has to be rejected
/// before that indexing rather than panicking.
#[test]
fn empty_list_is_rejected() {
    let processor = create_package_processor(MockForge::new(), None, None);

    assert!(
        processor
            .release_commit_message_for_pr_package_list(&[])
            .is_err()
    );
    assert!(processor.release_pr_title_for_pr_package_list(&[]).is_err());
}
