//! Rust workspace scenarios: shared `Cargo.lock`, virtual manifests,
//! root packages, and plain single crates.

use super::common::*;

/// Parent + two sub-packages: exactly one `Cargo.lock` change, carrying
/// every member's bump.
///
/// Both sub-packages list the workspace root's `Cargo.lock` among their
/// targets, as does the parent. Three passes over one file, each starting
/// from the same base content, would leave only the last bump.
#[tokio::test]
async fn a_shared_lock_is_written_once_with_every_member_bump() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "workspace"
path = "."
release_type = "rust"
sub_packages = [
  { name = "sub-a", path = "crates/a", release_type = "rust" },
  { name = "sub-b", path = "crates/b", release_type = "rust" },
]
"#,
        ),
        ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n"),
        (
            "Cargo.lock",
            "version = 3\n\n\
             [[package]]\nname = \"sub-a\"\nversion = \"0.0.1\"\n\n\
             [[package]]\nname = \"sub-b\"\nversion = \"0.0.1\"\n",
        ),
        (
            "crates/a/Cargo.toml",
            "[package]\nname = \"sub-a\"\nversion = \"0.0.1\"\n",
        ),
        (
            "crates/b/Cargo.toml",
            "[package]\nname = \"sub-b\"\nversion = \"0.0.1\"\n",
        ),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    let lock = change(&changes, "Cargo.lock");

    // Sub-packages share the parent's tag, so both land on the same
    // version.
    assert_lock_entry(&lock.content, "sub-a", "0.1.0");
    assert_lock_entry(&lock.content, "sub-b", "0.1.0");

    assert_eq!(count(&changes, "crates/a/Cargo.toml"), 1);
    assert_eq!(count(&changes, "crates/b/Cargo.toml"), 1);
}

/// A virtual manifest has no `[package]` table. Writing one via
/// `doc["package"]["version"]` auto-vivifies it, producing a manifest
/// declaring a package with nothing but a version — which cargo rejects.
#[tokio::test]
async fn a_virtual_manifest_never_gains_a_package_table() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "sub-a"
path = "crates/a"
workspace_root = "."
release_type = "rust"
"#,
        ),
        ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n"),
        (
            "crates/a/Cargo.toml",
            "[package]\nname = \"sub-a\"\nversion = \"0.0.1\"\n",
        ),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    assert_untouched(&changes, "Cargo.toml");
    assert!(
        change(&changes, "crates/a/Cargo.toml")
            .content
            .contains("version = \"0.1.0\"")
    );
}

/// `[workspace.dependencies]` in a virtual manifest is the one thing that
/// *must* be synced there: members referencing a peer with
/// `workspace = true` inherit the version from this table and from
/// nowhere else.
#[tokio::test]
async fn workspace_dependencies_sync_in_a_virtual_manifest() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "workspace"
path = "."
release_type = "rust"
sub_packages = [
  { name = "sub-a", path = "crates/a", release_type = "rust" },
  { name = "sub-b", path = "crates/b", release_type = "rust" },
]
"#,
        ),
        (
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\n\n\
             [workspace.dependencies]\n\
             sub-b = { path = \"crates/b\", version = \"0.0.1\" }\n",
        ),
        (
            "crates/a/Cargo.toml",
            "[package]\nname = \"sub-a\"\nversion = \"0.0.1\"\n\n\
             [dependencies]\nsub-b = { workspace = true }\n",
        ),
        (
            "crates/b/Cargo.toml",
            "[package]\nname = \"sub-b\"\nversion = \"0.0.1\"\n",
        ),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    let root = change(&changes, "Cargo.toml");
    assert!(
        root.content.contains("version = \"0.1.0\""),
        "{}",
        root.content
    );
    // Synced, not replaced: the path must survive.
    assert!(
        root.content.contains("path = \"crates/b\""),
        "{}",
        root.content
    );
    // Still no invented package.
    assert!(!root.content.contains("[package]"), "{}", root.content);

    // `sub-b = { workspace = true }` carries no version of its own and
    // must keep its shape.
    let sub_a = change(&changes, "crates/a/Cargo.toml");
    assert!(
        sub_a.content.contains("sub-b = { workspace = true }"),
        "{}",
        sub_a.content
    );
}

/// A root that is both `[package]` and `[workspace]` used to be skipped
/// wholesale, leaving the root crate permanently unbumped.
#[tokio::test]
async fn a_root_package_workspace_bumps_both_root_and_member() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "root"
path = "."
release_type = "rust"
sub_packages = [
  { name = "sub-a", path = "crates/a", release_type = "rust" },
]
"#,
        ),
        (
            "Cargo.toml",
            "[package]\nname = \"root\"\nversion = \"0.0.1\"\n\n\
             [workspace]\nmembers = [\"crates/*\"]\n",
        ),
        (
            "Cargo.lock",
            "version = 3\n\n\
             [[package]]\nname = \"root\"\nversion = \"0.0.1\"\n\n\
             [[package]]\nname = \"sub-a\"\nversion = \"0.0.1\"\n",
        ),
        (
            "crates/a/Cargo.toml",
            "[package]\nname = \"sub-a\"\nversion = \"0.0.1\"\n",
        ),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    let root = change(&changes, "Cargo.toml");
    assert!(
        root.content.contains("version = \"0.1.0\""),
        "{}",
        root.content
    );
    assert!(root.content.contains("members = [\"crates/*\"]"));

    let lock = change(&changes, "Cargo.lock");
    assert_lock_entry(&lock.content, "root", "0.1.0");
    assert_lock_entry(&lock.content, "sub-a", "0.1.0");
}

/// Two independent top-level packages sharing one workspace root. Dedup
/// has to span packages, not just one package's own target list.
#[tokio::test]
async fn sibling_top_level_packages_share_one_lock_change() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "pkg-a"
path = "crates/a"
workspace_root = "."
release_type = "rust"

[[package]]
name = "pkg-b"
path = "crates/b"
workspace_root = "."
release_type = "rust"
"#,
        ),
        ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n"),
        (
            "Cargo.lock",
            "version = 3\n\n\
             [[package]]\nname = \"pkg-a\"\nversion = \"0.0.1\"\n\n\
             [[package]]\nname = \"pkg-b\"\nversion = \"0.0.1\"\n",
        ),
        (
            "crates/a/Cargo.toml",
            "[package]\nname = \"pkg-a\"\nversion = \"0.0.1\"\n",
        ),
        (
            "crates/b/Cargo.toml",
            "[package]\nname = \"pkg-b\"\nversion = \"0.0.1\"\n",
        ),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    let lock = change(&changes, "Cargo.lock");
    assert_lock_entry(&lock.content, "pkg-a", "0.1.0");
    assert_lock_entry(&lock.content, "pkg-b", "0.1.0");

    assert_eq!(count(&changes, "crates/a/Cargo.toml"), 1);
    assert_eq!(count(&changes, "crates/b/Cargo.toml"), 1);
}

/// A plain crate with no `[workspace]` anywhere: the simplest case, and
/// the one most likely to break when workspace handling changes.
#[tokio::test]
async fn a_plain_crate_bumps_its_manifest_and_lock() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "solo"
path = "."
release_type = "rust"
"#,
        ),
        (
            "Cargo.toml",
            "[package]\nname = \"solo\"\nversion = \"0.0.1\"\n",
        ),
        (
            "Cargo.lock",
            "version = 3\n\n\
             [[package]]\nname = \"solo\"\nversion = \"0.0.1\"\n",
        ),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    assert!(
        change(&changes, "Cargo.toml")
            .content
            .contains("version = \"0.1.0\"")
    );
    assert_lock_entry(&change(&changes, "Cargo.lock").content, "solo", "0.1.0");
}

/// A sub-package's peers are its siblings as well as its parent. A
/// parent-only peer list silently leaves inter-member dependency versions
/// stale.
#[tokio::test]
async fn a_sub_package_dependency_on_a_sibling_is_bumped() {
    let scenario = Scenario::new(&[
        (
            "releasaurus.toml",
            r#"
[[package]]
name = "workspace"
path = "."
release_type = "rust"
sub_packages = [
  { name = "sub-a", path = "crates/a", release_type = "rust" },
  { name = "sub-b", path = "crates/b", release_type = "rust" },
]
"#,
        ),
        ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n"),
        (
            "crates/a/Cargo.toml",
            "[package]\nname = \"sub-a\"\nversion = \"0.0.1\"\n\n\
             [dependencies]\n\
             sub-b = { path = \"../b\", version = \"0.0.1\" }\n",
        ),
        (
            "crates/b/Cargo.toml",
            "[package]\nname = \"sub-b\"\nversion = \"0.0.1\"\n",
        ),
    ])
    .await
    .unwrap();

    let changes = scenario.file_changes().await.unwrap();

    let sub_a = change(&changes, "crates/a/Cargo.toml");
    let dep_line = sub_a
        .content
        .lines()
        .find(|l| l.starts_with("sub-b ="))
        .unwrap_or_else(|| panic!("no sub-b line in:\n{}", sub_a.content));

    assert!(dep_line.contains("version = \"0.1.0\""), "{dep_line}");
    assert!(dep_line.contains("path = \"../b\""), "{dep_line}");
}
