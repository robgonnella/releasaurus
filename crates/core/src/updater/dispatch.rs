//! Static dispatch updater enum for zero-cost abstraction over package updaters.

use std::path::Path;

use crate::{
    config::release_type::ReleaseType,
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{
        generic::updater::GenericUpdater,
        go::{manifests::GoManifests, updater::GoUpdater},
        java::{manifests::JavaManifests, updater::JavaUpdater},
        manager::ManifestTarget,
        node::{manifests::NodeManifests, updater::NodeUpdater},
        php::{manifests::PhpManifests, updater::PhpUpdater},
        python::{manifests::PythonManifests, updater::PythonUpdater},
        ruby::{manifests::RubyManifests, updater::RubyUpdater},
        rust::{manifests::RustManifests, updater::RustUpdater},
        traits::{FileUpdater, ManifestTargets},
    },
};

/// Language-specific updater with static dispatch for optimal performance.
///
/// This enum wraps concrete updater implementations, allowing the compiler to
/// use static dispatch instead of dynamic dispatch (vtable lookups), resulting
/// in better inlining and optimization opportunities.
pub enum Updater {
    /// Generic updater for projects without specific language support
    Generic(GenericUpdater),
    /// Golang updater for version.go files
    Go(GoUpdater),
    /// Java/Maven updater for pom.xml files
    Java(JavaUpdater),
    /// Node.js updater for package.json, package-lock.json, and yarn.lock
    Node(NodeUpdater),
    /// PHP updater for composer.json
    Php(PhpUpdater),
    /// Python updater for setup.py, pyproject.toml, etc.
    Python(PythonUpdater),
    /// Ruby updater for Gemfile, gemspec, version.rb
    Ruby(RubyUpdater),
    /// Rust updater for Cargo.toml and Cargo.lock
    Rust(RustUpdater),
}

impl Updater {
    /// Create a new updater instance for the given release type.
    pub fn new(release_type: ReleaseType) -> Self {
        match release_type {
            ReleaseType::Generic => Updater::Generic(GenericUpdater::default()),
            ReleaseType::Go => Updater::Go(GoUpdater::new()),
            ReleaseType::Java => Updater::Java(JavaUpdater::new()),
            ReleaseType::Node => Updater::Node(NodeUpdater::new()),
            ReleaseType::Php => Updater::Php(PhpUpdater::new()),
            ReleaseType::Python => Updater::Python(PythonUpdater::new()),
            ReleaseType::Ruby => Updater::Ruby(RubyUpdater::new()),
            ReleaseType::Rust => Updater::Rust(RustUpdater::new()),
        }
    }

    pub fn manifest_targets(
        &self,
        pkg_name: &str,
        workspace_path: &Path,
        pkg_path: &Path,
    ) -> Vec<ManifestTarget> {
        match self {
            Updater::Generic(_) => vec![],
            Updater::Go(_) => GoManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
            Updater::Java(_) => JavaManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
            Updater::Node(_) => NodeManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
            Updater::Php(_) => PhpManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
            Updater::Python(_) => PythonManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
            Updater::Ruby(_) => RubyManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
            Updater::Rust(_) => RustManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
        }
    }

    /// Update package version files with static dispatch.
    ///
    /// This method dispatches to the appropriate language-specific updater
    /// using static dispatch, avoiding the overhead of trait objects and
    /// enabling compiler optimizations like inlining.
    pub fn update(
        &self,
        manifest: &ManifestFile,
    ) -> Result<Option<FileChange>> {
        match self {
            Updater::Generic(updater) => updater.update(manifest),
            Updater::Go(updater) => updater.update(manifest),
            Updater::Java(updater) => updater.update(manifest),
            Updater::Node(updater) => updater.update(manifest),
            Updater::Php(updater) => updater.update(manifest),
            Updater::Python(updater) => updater.update(manifest),
            Updater::Ruby(updater) => updater.update(manifest),
            Updater::Rust(updater) => updater.update(manifest),
        }
    }

    /// Update every manifest of this release type in one pass.
    ///
    /// Preferred over [`Updater::update`] for whole-package work: an
    /// updater whose files depend on each other only sees that
    /// relationship here.
    pub fn update_all(
        &self,
        manifests: &[ManifestFile],
    ) -> Result<Vec<FileChange>> {
        match self {
            Updater::Generic(updater) => updater.update_all(manifests),
            Updater::Go(updater) => updater.update_all(manifests),
            Updater::Java(updater) => updater.update_all(manifests),
            Updater::Node(updater) => updater.update_all(manifests),
            Updater::Php(updater) => updater.update_all(manifests),
            Updater::Python(updater) => updater.update_all(manifests),
            Updater::Ruby(updater) => updater.update_all(manifests),
            Updater::Rust(updater) => updater.update_all(manifests),
        }
    }
}

impl std::fmt::Debug for Updater {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Updater::Generic(_) => write!(f, "Updater::Generic"),
            Updater::Go(_) => write!(f, "Updater::Go"),
            Updater::Java(_) => write!(f, "Updater::Java"),
            Updater::Node(_) => write!(f, "Updater::Node"),
            Updater::Php(_) => write!(f, "Updater::Php"),
            Updater::Python(_) => write!(f, "Updater::Python"),
            Updater::Ruby(_) => write!(f, "Updater::Ruby"),
            Updater::Rust(_) => write!(f, "Updater::Rust"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_updater_for_each_release_type() {
        let types = vec![
            ReleaseType::Generic,
            ReleaseType::Java,
            ReleaseType::Node,
            ReleaseType::Php,
            ReleaseType::Python,
            ReleaseType::Ruby,
            ReleaseType::Rust,
        ];

        for release_type in types {
            let updater = Updater::new(release_type);
            // If we got here without panicking, the updater was created successfully
            assert!(matches!(
                updater,
                Updater::Generic(_)
                    | Updater::Java(_)
                    | Updater::Node(_)
                    | Updater::Php(_)
                    | Updater::Python(_)
                    | Updater::Ruby(_)
                    | Updater::Rust(_)
            ));
        }
    }
}
