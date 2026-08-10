//! PHP scenarios. `composer.lock` is the one manifest whose new content is
//! not a function of its own old content: its `content-hash` is an md5 over
//! the relevant keys of the *updated* `composer.json` beside it, so these
//! scenarios pin the hash rather than just checking a change was emitted.

use super::common::*;

/// The `content-hash` a lock carries after a release.
fn content_hash(
    changes: &[crate::forge::request::FileChange],
    path: &str,
) -> String {
    let doc: serde_json::Value =
        serde_json::from_str(&change(changes, path).content)
            .expect("lock is json");

    doc["content-hash"]
        .as_str()
        .unwrap_or_else(|| panic!("no content-hash in {path}"))
        .to_string()
}

/// The lock's hash must describe the bumped `composer.json`, not the one on
/// the branch. A hash taken from the pre-bump file is still well-formed, so
/// only the literal value catches it — composer would reject the lock as
/// stale at install time.
#[tokio::test]
async fn the_lock_hash_covers_the_bumped_composer_json() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "pkg-a"
path = "."
release_type = "php"
"#,
        ),
        (
            "composer.json",
            r#"{"name":"vendor/pkg-a","version":"0.0.1","require":{"php":">=8.1"}}"#,
        ),
        (
            "composer.lock",
            r#"{"content-hash":"stale","packages":[],"packages-dev":[]}"#,
        ),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    assert!(
        change(&changes, "composer.json")
            .content
            .contains("\"version\": \"0.1.0\""),
    );

    // md5 of {"name":"vendor\/pkg-a","require":{"php":">=8.1"},"version":"0.1.0"}
    assert_eq!(
        content_hash(&changes, "composer.lock"),
        "0d9a082f1c8a2d81c36bbf0ea8b20356",
    );
}

/// Two PHP packages on one release branch. Each lock has to be hashed
/// against the `composer.json` in its own directory — the names differ, so
/// crossing them produces the other's hash.
#[tokio::test]
async fn each_package_lock_is_hashed_from_its_own_composer_json() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "pkg-a"
path = "packages/a"
release_type = "php"

[[package]]
name = "pkg-b"
path = "packages/b"
release_type = "php"
"#,
        ),
        (
            "packages/a/composer.json",
            r#"{"name":"vendor/pkg-a","version":"0.0.1","require":{"php":">=8.1"}}"#,
        ),
        (
            "packages/a/composer.lock",
            r#"{"content-hash":"stale-a","packages":[],"packages-dev":[]}"#,
        ),
        (
            "packages/b/composer.json",
            r#"{"name":"vendor/pkg-b","version":"0.0.1"}"#,
        ),
        (
            "packages/b/composer.lock",
            r#"{"content-hash":"stale-b","packages":[],"packages-dev":[]}"#,
        ),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    assert_eq!(
        content_hash(&changes, "packages/a/composer.lock"),
        "0d9a082f1c8a2d81c36bbf0ea8b20356",
    );
    assert_eq!(
        content_hash(&changes, "packages/b/composer.lock"),
        "d427413f97139a732f35712c355831ce",
    );
}

/// A package with no committed lock releases normally rather than failing
/// on a missing sibling.
#[tokio::test]
async fn a_package_without_a_lock_still_releases() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "pkg-a"
path = "."
release_type = "php"
"#,
        ),
        (
            "composer.json",
            r#"{"name":"vendor/pkg-a","version":"0.0.1"}"#,
        ),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    assert!(
        change(&changes, "composer.json")
            .content
            .contains("\"version\": \"0.1.0\""),
    );
    assert_untouched(&changes, "composer.lock");
}
