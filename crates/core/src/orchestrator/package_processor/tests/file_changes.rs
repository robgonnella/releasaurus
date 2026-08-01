//! Tests for file change generation.
//!
//! Tests for:
//! - Which other releasable packages a package cross-references when
//!   its manifests declare them as dependencies

use semver::Version;

use super::common::*;

use crate::{
    config::{
        package::{PackageConfig, PackageConfigBuilder},
        release_type::ReleaseType,
    },
    forge::{request::Tag, traits::MockForge},
    packages::{manifests::ManifestFile, releasable::ReleasablePackage},
};

/// A `package.json` for `pkg-a` that depends on `pkg-b`.
const PKG_A_MANIFEST: &str = r#"{
  "name": "pkg-a",
  "version": "1.0.0",
  "dependencies": { "pkg-b": "1.0.0" }
}"#;

/// Builds a releasable package carrying `content` as its manifest.
fn releasable_with_manifest(
    name: &str,
    path: &str,
    tag: &str,
    release_type: ReleaseType,
    basename: &str,
    content: &str,
) -> ReleasablePackage {
    ReleasablePackage {
        name: name.into(),
        release_type,
        tag: Tag {
            name: tag.into(),
            semver: Version::parse(tag.trim_start_matches('v')).unwrap(),
            ..Default::default()
        },
        notes: format!("notes for {name}"),
        manifest_files: Some(vec![ManifestFile {
            path: format!("{path}/{basename}").into(),
            basename: basename.into(),
            content: content.into(),
        }]),
        ..Default::default()
    }
}

fn node_package(
    name: &str,
    path: &str,
    tag: &str,
    content: &str,
) -> ReleasablePackage {
    releasable_with_manifest(
        name,
        path,
        tag,
        ReleaseType::Node,
        "package.json",
        content,
    )
}

fn package_config(
    name: &str,
    workspace_root: &str,
    release_type: ReleaseType,
) -> PackageConfig {
    PackageConfigBuilder::default()
        .name(name)
        .workspace_root(workspace_root)
        .path(format!("{workspace_root}/{name}"))
        .release_type(release_type)
        .build()
        .unwrap()
}

/// Extracts the sole manifest file change from a generated change set.
fn manifest_change(changes: &[crate::forge::request::FileChange]) -> String {
    changes
        .iter()
        .find(|c| c.path.ends_with("package.json"))
        .expect("expected a package.json file change")
        .content
        .clone()
}

/// A package in one workspace can depend on a package in another, so
/// the bump has to cross the workspace boundary.
#[test]
fn file_changes_bump_a_declared_dependency_across_workspaces() {
    let processor = create_package_processor(
        MockForge::new(),
        Some(vec![
            package_config("pkg-a", "workspace-one", ReleaseType::Node),
            package_config("pkg-b", "workspace-two", ReleaseType::Node),
        ]),
        None,
    );

    let releasable = vec![
        node_package("pkg-a", "workspace-one/pkg-a", "v2.0.0", PKG_A_MANIFEST),
        node_package("pkg-b", "workspace-two/pkg-b", "v3.0.0", "{}"),
    ];

    let content = manifest_change(
        &processor
            .file_changes_for_releasable_package(&releasable[0], &releasable)
            .unwrap(),
    );

    assert!(
        content.contains("\"version\": \"2.0.0\""),
        "pkg-a should get its own bump: {content}"
    );
    assert!(
        content.contains("\"pkg-b\": \"^3.0.0\""),
        "pkg-b is declared as a dependency and should be bumped even from \
         another workspace: {content}"
    );
}

/// The same layout within one workspace, for contrast.
#[test]
fn file_changes_bump_a_declared_dependency_in_the_same_workspace() {
    let processor = create_package_processor(
        MockForge::new(),
        Some(vec![
            package_config("pkg-a", "workspace-one", ReleaseType::Node),
            package_config("pkg-b", "workspace-one", ReleaseType::Node),
        ]),
        None,
    );

    let releasable = vec![
        node_package("pkg-a", "workspace-one/pkg-a", "v2.0.0", PKG_A_MANIFEST),
        node_package("pkg-b", "workspace-one/pkg-b", "v3.0.0", "{}"),
    ];

    let content = manifest_change(
        &processor
            .file_changes_for_releasable_package(&releasable[0], &releasable)
            .unwrap(),
    );

    assert!(
        content.contains("\"pkg-b\": \"^3.0.0\""),
        "pkg-b shares the workspace and should be bumped: {content}"
    );
}

/// A Rust package's version must never land in a Node manifest.
#[test]
fn file_changes_ignore_packages_of_a_different_release_type() {
    let processor = create_package_processor(
        MockForge::new(),
        Some(vec![
            package_config("pkg-a", "workspace-one", ReleaseType::Node),
            package_config("pkg-b", "workspace-one", ReleaseType::Rust),
        ]),
        None,
    );

    let releasable = vec![
        node_package("pkg-a", "workspace-one/pkg-a", "v2.0.0", PKG_A_MANIFEST),
        releasable_with_manifest(
            "pkg-b",
            "workspace-one/pkg-b",
            "v3.0.0",
            ReleaseType::Rust,
            "Cargo.toml",
            "[package]\nname = \"pkg-b\"\nversion = \"1.0.0\"\n",
        ),
    ];

    let content = manifest_change(
        &processor
            .file_changes_for_releasable_package(&releasable[0], &releasable)
            .unwrap(),
    );

    assert!(
        content.contains("\"pkg-b\": \"1.0.0\""),
        "the Rust pkg-b must not bump a Node dependency of the same name: \
         {content}"
    );
}
