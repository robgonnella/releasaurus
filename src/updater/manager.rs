//! Framework and package management for multi-language support.
use log::*;
use std::path::Path;

use crate::Result;
use crate::analyzer::release::Tag;
use crate::cli::types::ReleasablePackage;
use crate::config::package::PackageConfig;
use crate::config::release_type::ReleaseType;
use crate::forge::request::FileChange;
use crate::path_helpers::package_path;
use crate::updater::{
    generic::updater::GenericUpdater,
    java::{manifests::JavaManifests, updater::JavaUpdater},
    node::{manifests::NodeManifests, updater::NodeUpdater},
    php::{manifests::PhpManifests, updater::PhpUpdater},
    python::{manifests::PythonManifests, updater::PythonUpdater},
    ruby::{manifests::RubyManifests, updater::RubyUpdater},
    rust::{manifests::RustManifests, updater::RustUpdater},
    traits::{ManifestTargets, PackageUpdater},
};

#[derive(Debug)]
pub struct ManifestTarget {
    /// Whether or not to treat this as a workspace manifest
    pub is_workspace: bool,
    /// The file path relative to the package path
    pub path: String,
    /// The base name of the file path
    pub basename: String,
}

#[derive(Default, Clone)]
pub struct ManifestFile {
    /// Whether or not to treat this as a workspace manifest
    pub is_workspace: bool,
    /// The file path relative to the package path
    pub path: String,
    /// The base name of the file path
    pub basename: String,
    /// The current content of the file
    pub content: String,
}

impl std::fmt::Debug for ManifestFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManifestFile")
            .field("is_workspace", &self.is_workspace)
            .field("path", &self.path)
            .field("basename", &self.basename)
            .finish()
    }
}

/// Programming language and package manager detection for determining which
/// version files to update.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UpdateManager {}

impl UpdateManager {
    pub fn release_type_manifest_targets(
        pkg: &PackageConfig,
    ) -> Vec<ManifestTarget> {
        match pkg.release_type.clone() {
            Some(ReleaseType::Generic) => vec![],
            Some(ReleaseType::Java) => JavaManifests::manifest_targets(pkg),
            Some(ReleaseType::Node) => NodeManifests::manifest_targets(pkg),
            Some(ReleaseType::Php) => PhpManifests::manifest_targets(pkg),
            Some(ReleaseType::Python) => PythonManifests::manifest_targets(pkg),
            Some(ReleaseType::Ruby) => RubyManifests::manifest_targets(pkg),
            Some(ReleaseType::Rust) => RustManifests::manifest_targets(pkg),
            None => vec![],
        }
    }

    pub fn additional_manifest_targets(
        pkg: &PackageConfig,
    ) -> Vec<ManifestTarget> {
        if let Some(additional) = pkg.additional_manifest_files.clone() {
            let mut targets = vec![];

            for manifest_path in additional {
                let basename = Path::new(&manifest_path)
                    .file_name()
                    .map(|f| f.display().to_string())
                    .unwrap_or(manifest_path.clone());

                targets.push(ManifestTarget {
                    is_workspace: false,
                    path: package_path(pkg, Some(&manifest_path)),
                    basename,
                })
            }

            targets
        } else {
            vec![]
        }
    }

    pub fn get_package_manifest_file_changes(
        package: &ReleasablePackage,
        all_packages: &[ReleasablePackage],
    ) -> Result<Vec<FileChange>> {
        let mut file_changes = vec![];

        // gather other packages related to target package that may be in
        // same workspace
        let workspace_packages: Vec<_> = all_packages
            .iter()
            .filter(|p| {
                p.name != package.name
                    && p.workspace_root == package.workspace_root
                    && p.release_type == package.release_type
            })
            .collect();

        let updater_package = UpdaterPackage::from_releasable_package(package);

        let workspace_updater_packages = workspace_packages
            .into_iter()
            .map(UpdaterPackage::from_releasable_package)
            .collect::<Vec<UpdaterPackage>>();

        info!(
            "Package: {}: Found {} other packages for workspace root: {}, release_type: {}",
            updater_package.package_name,
            workspace_updater_packages.len(),
            package.workspace_root,
            updater_package.release_type
        );

        let updater = UpdateManager::updater(package.release_type.clone());

        if let Some(changes) =
            updater.update(&updater_package, workspace_updater_packages)?
        {
            file_changes.extend(changes);
        }

        if let Some(additional) = package.additional_manifest_files.clone() {
            for manifest in additional.iter() {
                if let Some(change) = GenericUpdater::update_manifest(
                    manifest,
                    &updater_package.next_version.semver,
                ) {
                    file_changes.push(change);
                }
            }
        }

        Ok(file_changes)
    }

    /// Get language-specific updater implementation for this framework.
    fn updater(release_type: ReleaseType) -> Box<dyn PackageUpdater> {
        match release_type {
            ReleaseType::Generic => Box::new(GenericUpdater::new()),
            ReleaseType::Java => Box::new(JavaUpdater::new()),
            ReleaseType::Node => Box::new(NodeUpdater::new()),
            ReleaseType::Php => Box::new(PhpUpdater::new()),
            ReleaseType::Python => Box::new(PythonUpdater::new()),
            ReleaseType::Ruby => Box::new(RubyUpdater::new()),
            ReleaseType::Rust => Box::new(RustUpdater::new()),
        }
    }
}

/// Package information with next version and framework details for version
/// file updates.
#[derive(Debug, Clone)]
pub struct UpdaterPackage {
    /// Package name derived from manifest or directory.
    pub package_name: String,
    /// Path to the workspace root directory for this package relative to the repository root
    // pub workspace_root: String,
    /// List of manifest files to update
    pub manifest_files: Vec<ManifestFile>,
    /// Next version to update to based on commit analysis.
    pub next_version: Tag,
    /// [`ReleaseType`] for selecting appropriate updater.
    pub release_type: ReleaseType,
}

impl UpdaterPackage {
    fn from_releasable_package(pkg: &ReleasablePackage) -> Self {
        let tag = pkg.release.tag.clone().unwrap_or_default();

        UpdaterPackage {
            package_name: pkg.name.clone(),
            // workspace_root: pkg.workspace_root.clone(),
            release_type: pkg.release_type.clone(),
            manifest_files: pkg.manifest_files.clone().unwrap_or_default(),
            next_version: tag,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_package(release_type: Option<ReleaseType>) -> PackageConfig {
        PackageConfig {
            name: "test-package".to_string(),
            workspace_root: ".".to_string(),
            path: ".".to_string(),
            release_type,
            ..Default::default()
        }
    }

    #[test]
    fn release_type_manifest_targets_returns_empty_for_generic() {
        let pkg = create_test_package(Some(ReleaseType::Generic));
        let targets = UpdateManager::release_type_manifest_targets(&pkg);
        assert_eq!(targets.len(), 0);
    }

    #[test]
    fn release_type_manifest_targets_returns_empty_for_none() {
        let pkg = create_test_package(None);
        let targets = UpdateManager::release_type_manifest_targets(&pkg);
        assert_eq!(targets.len(), 0);
    }

    #[test]
    fn release_type_manifest_targets_delegates_to_language_manifests() {
        let test_cases = vec![
            (ReleaseType::Java, 6),
            (ReleaseType::Node, 3),
            (ReleaseType::Php, 1),
            (ReleaseType::Python, 3),
            (ReleaseType::Ruby, 4),
            (ReleaseType::Rust, 2),
        ];

        for (release_type, expected_count) in test_cases {
            let pkg = create_test_package(Some(release_type.clone()));
            let targets = UpdateManager::release_type_manifest_targets(&pkg);
            assert_eq!(
                targets.len(),
                expected_count,
                "Expected {} targets for {:?}",
                expected_count,
                release_type
            );
        }
    }

    #[test]
    fn additional_manifest_targets_returns_empty_when_none_configured() {
        let pkg = create_test_package(Some(ReleaseType::Node));
        let targets = UpdateManager::additional_manifest_targets(&pkg);
        assert_eq!(targets.len(), 0);
    }

    #[test]
    fn additional_manifest_targets_returns_configured_files() {
        let mut pkg = create_test_package(Some(ReleaseType::Generic));
        pkg.path = "packages/my-app".to_string();
        pkg.additional_manifest_files =
            Some(vec!["VERSION".to_string(), "config/app.yml".to_string()]);

        let targets = UpdateManager::additional_manifest_targets(&pkg);

        assert_eq!(targets.len(), 2);
        assert!(targets.iter().all(|t| !t.is_workspace));

        let basenames: Vec<_> = targets.iter().map(|t| &t.basename).collect();
        assert!(basenames.contains(&&"VERSION".to_string()));
        assert!(basenames.contains(&&"app.yml".to_string()));

        let paths: Vec<_> = targets.iter().map(|t| t.path.as_str()).collect();
        assert!(paths.contains(&"packages/my-app/VERSION"));
        assert!(paths.contains(&"packages/my-app/config/app.yml"));
    }
}
