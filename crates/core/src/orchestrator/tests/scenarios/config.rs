//! Config shapes that must be rejected before the pipeline runs.
//!
//! Ownership is derived from a package's directory, so two packages on one
//! path make it ambiguous — and they would both write the same
//! `CHANGELOG.md`, with one silently dropped.

use super::common::*;
use crate::result::ReleasaurusError;

#[tokio::test]
async fn two_packages_on_one_path_are_rejected() {
    let err = expect_rejected(
        Scenario::new(&[
            (
                "releasaurus.toml",
                r#"
[[package]]
name = "pkg-a"
path = "crates/shared"
release_type = "rust"

[[package]]
name = "pkg-b"
path = "crates/shared"
release_type = "rust"
"#,
            ),
            (
                "crates/shared/Cargo.toml",
                "[package]\nname = \"pkg-a\"\nversion = \"0.0.1\"\n",
            ),
        ])
        .await,
    );

    assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));
}

/// A sub-package sharing its parent's directory is the same ambiguity,
/// reached through a different config shape.
#[tokio::test]
async fn a_sub_package_on_its_parents_path_is_rejected() {
    let err = expect_rejected(
        Scenario::new(&[
            (
                "releasaurus.toml",
                r#"
[[package]]
name = "parent"
path = "."
release_type = "rust"
sub_packages = [
  { name = "child", path = ".", release_type = "rust" },
]
"#,
            ),
            (
                "Cargo.toml",
                "[package]\nname = \"parent\"\nversion = \"0.0.1\"\n",
            ),
        ])
        .await,
    );

    assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));
}

/// Distinct paths are fine, including a root package with nested
/// sub-packages — the layout this repo itself uses.
#[tokio::test]
async fn a_root_package_with_nested_sub_packages_is_accepted() {
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

    assert_eq!(count(&changes, "crates/a/Cargo.toml"), 1);
    assert_eq!(count(&changes, "crates/b/Cargo.toml"), 1);
}
