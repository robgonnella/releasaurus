//! Node workspace scenarios, where the root `package-lock.json` has an
//! owner only when the root itself is being released.

use super::common::*;

/// An npm workspace with no released root. The lock's own `version` and
/// its `packages[""]` entry describe the *root* package, so a member's
/// version must not land in either — while `node_modules/*` entries for
/// released members must still be bumped.
#[tokio::test]
async fn an_unowned_root_lock_keeps_its_own_version() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "pkg-a"
path = "packages/a"
workspace_root = "."
release_type = "node"

[[package]]
name = "pkg-b"
path = "packages/b"
workspace_root = "."
release_type = "node"
"#,
        ),
        (
            "package-lock.json",
            r#"{
  "name": "root",
  "version": "9.9.9",
  "packages": {
    "": {
      "name": "root",
      "version": "9.9.9"
    },
    "node_modules/pkg-a": {
      "version": "0.0.1"
    },
    "node_modules/pkg-b": {
      "version": "0.0.1"
    }
  }
}"#,
        ),
        (
            "packages/a/package.json",
            r#"{"name":"pkg-a","version":"0.0.1"}"#,
        ),
        (
            "packages/b/package.json",
            r#"{"name":"pkg-b","version":"0.0.1"}"#,
        ),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    let lock = change(&changes, "package-lock.json");

    // Nobody released the root, so its version is left alone.
    assert_eq!(
        lock.content.matches("\"version\": \"9.9.9\"").count(),
        2,
        "root version and packages[\"\"] should both be untouched:\n{}",
        lock.content
    );

    // Members still get bumped.
    assert_eq!(
        lock.content.matches("\"version\": \"0.1.0\"").count(),
        2,
        "both node_modules entries should be bumped:\n{}",
        lock.content
    );
}

/// The root `package.json` is only ever a target when the root is itself
/// the package, so a member can never stamp its version there.
#[tokio::test]
async fn a_member_never_writes_the_root_package_json() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "pkg-a"
path = "packages/a"
workspace_root = "."
release_type = "node"
"#,
        ),
        (
            "package.json",
            r#"{"name":"root","version":"9.9.9","workspaces":["packages/*"]}"#,
        ),
        (
            "packages/a/package.json",
            r#"{"name":"pkg-a","version":"0.0.1"}"#,
        ),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    assert_untouched(&changes, "package.json");
    assert!(
        change(&changes, "packages/a/package.json")
            .content
            .contains("\"version\": \"0.1.0\"")
    );
}
