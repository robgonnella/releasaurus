//! Framework and package management for multi-language support.
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::{
    config::release_type::ReleaseType,
    forge::{
        request::{FileChange, Tag},
        traits::FileLoader,
    },
    packages::{
        manifests::{AdditionalManifestFile, ManifestFile},
        releasable::ReleasablePackage,
        resolved::ResolvedPackage,
    },
    result::Result,
    updater::{
        dispatch::Updater, generic::updater::GenericUpdater,
        go::manifests::GoManifests, java::manifests::JavaManifests,
        node::manifests::NodeManifests, php::manifests::PhpManifests,
        python::manifests::PythonManifests, ruby::manifests::RubyManifests,
        rust::manifests::RustManifests, traits::ManifestTargets,
    },
};

/// A single manifest file path to load from the forge.
#[derive(Debug)]
pub struct ManifestTarget {
    /// The file path relative to the package path
    pub path: PathBuf,
    /// The base name of the file path
    pub basename: String,
}

/// Programming language and package manager detection for determining which
/// version files to update.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct UpdateManager {}

impl UpdateManager {
    /// Load manifest files for a package using a FileLoader.
    ///
    /// This method orchestrates the loading of all manifest files needed for
    /// a package by determining which files are needed (via manifest_targets)
    /// and then loading their content using the provided FileLoader.
    pub async fn load_manifests_for_package<F: FileLoader>(
        pkg: &ResolvedPackage,
        file_loader: &F,
        base_branch: &str,
    ) -> Result<Option<Vec<ManifestFile>>> {
        let targets = Self::release_type_manifest_targets(
            &pkg.name,
            pkg.release_type,
            &pkg.normalized_workspace_root,
            &pkg.normalized_full_path,
        );

        if targets.is_empty() {
            return Ok(None);
        }

        let mut manifests = vec![];

        for target in targets {
            log::debug!(
                "Loading manifest target: {}",
                target.path.to_string_lossy()
            );
            if let Some(content) = file_loader
                .load_file(
                    Some(base_branch.into()),
                    target.path.to_string_lossy().to_string(),
                )
                .await?
            {
                log::info!(
                    "Loaded manifest: {}",
                    target.path.to_string_lossy()
                );
                manifests.push(ManifestFile {
                    path: target.path,
                    basename: target.basename,
                    content,
                });
            } else {
                log::debug!(
                    "Manifest not found: {}",
                    target.path.to_string_lossy()
                );
            }
        }

        if manifests.is_empty() {
            Ok(None)
        } else {
            Ok(Some(manifests))
        }
    }

    /// Load additional manifest files for a package using a FileLoader.
    ///
    /// Loads user-configured additional manifest files that are not part of
    /// the standard release type manifests.
    pub async fn load_additional_manifests_for_package<F: FileLoader>(
        pkg: &ResolvedPackage,
        file_loader: &F,
        branch: &str,
    ) -> Result<Option<Vec<AdditionalManifestFile>>> {
        if pkg.compiled_additional_manifests.is_empty() {
            return Ok(None);
        }

        let mut manifests = vec![];

        for compiled in &pkg.compiled_additional_manifests {
            let basename = compiled
                .path
                .file_name()
                .map(|f| f.to_string_lossy())
                .unwrap_or(compiled.path.to_string_lossy())
                .into();

            log::debug!(
                "Loading additional manifest: {}",
                compiled.path.to_string_lossy()
            );

            if let Some(content) = file_loader
                .load_file(
                    Some(branch.into()),
                    compiled.path.to_string_lossy().to_string(),
                )
                .await?
            {
                log::info!(
                    "Loaded additional manifest: {}",
                    compiled.path.to_string_lossy()
                );

                manifests.push(AdditionalManifestFile {
                    path: compiled.path.clone(),
                    basename,
                    content,
                    version_regex: compiled.version_regex.clone(),
                });
            } else {
                log::warn!(
                    "Additional manifest not found: {}",
                    compiled.path.to_string_lossy()
                );
            }
        }

        if manifests.is_empty() {
            Ok(None)
        } else {
            Ok(Some(manifests))
        }
    }

    /// Generate all file changes needed to bump versions for a
    /// package.
    ///
    /// Handles the primary manifest, user-configured additional
    /// manifests, and sub-packages (for workspace-style repos).
    pub fn get_package_manifest_file_changes(
        package: &ReleasablePackage,
        workspace_packages: &[&ReleasablePackage],
    ) -> Result<Vec<FileChange>> {
        let mut file_changes = vec![];

        let updater_package = UpdaterPackage::from_releasable_package(package);

        let workspace_updater_packages = workspace_packages
            .iter()
            .map(|&p| UpdaterPackage::from_releasable_package(p))
            .collect::<Vec<UpdaterPackage>>();

        if let Some(changes) = updater_package
            .updater
            .update(&updater_package, &workspace_updater_packages)?
        {
            file_changes.extend(changes);
        }

        if let Some(additional) = package.additional_manifest_files.clone() {
            for manifest in additional.iter() {
                if let Some(change) = GenericUpdater::update_manifest(
                    &manifest.into(),
                    &updater_package.next_version.semver,
                    &manifest.version_regex,
                ) {
                    file_changes.push(change);
                }
            }
        }

        let sub_packages: Vec<_> = package
            .sub_packages
            .iter()
            .map(|s| s.to_releasable(package))
            .collect();

        if !sub_packages.is_empty() {
            // Build workspace context including both sub-packages and
            // parent. This is needed for workspace-level manifest updates
            // (e.g., Cargo.lock) Pre-allocate to avoid reallocation during
            // push
            let mut workspace_refs: Vec<&ReleasablePackage> =
                Vec::with_capacity(package.sub_packages.len() + 1);
            workspace_refs.extend(sub_packages.iter());
            workspace_refs.push(package);

            for sub in sub_packages.iter() {
                file_changes.extend(
                    UpdateManager::get_package_manifest_file_changes(
                        sub,
                        &workspace_refs,
                    )?,
                )
            }
        }

        Ok(file_changes)
    }

    fn release_type_manifest_targets(
        pkg_name: &str,
        release_type: ReleaseType,
        workspace_path: &Path,
        pkg_path: &Path,
    ) -> Vec<ManifestTarget> {
        match release_type {
            ReleaseType::Generic => vec![],
            ReleaseType::Go => GoManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
            ReleaseType::Java => JavaManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
            ReleaseType::Node => NodeManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
            ReleaseType::Php => PhpManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
            ReleaseType::Python => PythonManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
            ReleaseType::Ruby => RubyManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
            ReleaseType::Rust => RustManifests::manifest_targets(
                pkg_name,
                workspace_path,
                pkg_path,
            ),
        }
    }
}

/// Package information with next version and framework details for version
/// file updates.
#[derive(Debug, Clone)]
pub struct UpdaterPackage {
    /// Package name derived from manifest or directory.
    pub package_name: String,
    /// List of manifest files to update
    pub manifest_files: Vec<ManifestFile>,
    /// Next version to update to based on commit analysis.
    pub next_version: Tag,
    /// Pre-built updater instance for this package type, created once and
    /// reused. Wrapped in Rc for cheap cloning across workspace packages.
    pub updater: Rc<Updater>,
}

impl UpdaterPackage {
    fn from_releasable_package(pkg: &ReleasablePackage) -> Self {
        let updater = Rc::new(Updater::new(pkg.release_type));

        UpdaterPackage {
            package_name: pkg.name.clone(),
            manifest_files: pkg
                .manifest_files
                .as_ref()
                .cloned()
                .unwrap_or_default(),
            next_version: pkg.tag.clone(),
            updater,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn create_test_manifest_params() -> (String, PathBuf, PathBuf) {
        let pkg_name = "test-package".to_string();
        let workspace_path = Path::new("").to_path_buf();
        let pkg_path = workspace_path.clone();
        (pkg_name, workspace_path, pkg_path)
    }

    #[test]
    fn release_type_manifest_targets_returns_empty_for_generic() {
        let release_type = ReleaseType::Generic;
        let (pkg_name, workspace_path, pkg_path) =
            create_test_manifest_params();
        let targets = UpdateManager::release_type_manifest_targets(
            &pkg_name,
            release_type,
            &workspace_path,
            &pkg_path,
        );
        assert_eq!(targets.len(), 0);
    }

    #[test]
    fn release_type_manifest_targets_delegates_to_language_manifests() {
        let test_cases = vec![
            (ReleaseType::Java, 7),
            (ReleaseType::Node, 3),
            (ReleaseType::Php, 2),
            (ReleaseType::Python, 3),
            (ReleaseType::Ruby, 4),
            (ReleaseType::Rust, 2),
        ];

        for (release_type, expected_count) in test_cases {
            let (pkg_name, workspace_path, pkg_path) =
                create_test_manifest_params();

            let targets = UpdateManager::release_type_manifest_targets(
                &pkg_name,
                release_type,
                &workspace_path,
                &pkg_path,
            );

            assert_eq!(
                targets.len(),
                expected_count,
                "Expected {} targets for {:?}",
                expected_count,
                release_type
            );
        }
    }
}
