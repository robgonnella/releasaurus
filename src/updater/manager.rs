//! Framework and package management for multi-language support.
use regex::Regex;
use serde::Serialize;
use serde::ser::SerializeStruct;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::analyzer::release::Tag;
use crate::config::release_type::ReleaseType;
use crate::file_loader::FileLoader;
use crate::forge::request::FileChange;
use crate::orchestrator::package::releasable::ReleasablePackage;
use crate::updater::generic::updater::GENERIC_VERSION_REGEX;
use crate::updater::go::manifests::GoManifests;
use crate::updater::{
    dispatch::Updater, generic::updater::GenericUpdater,
    java::manifests::JavaManifests, node::manifests::NodeManifests,
    php::manifests::PhpManifests, python::manifests::PythonManifests,
    ruby::manifests::RubyManifests, rust::manifests::RustManifests,
    traits::ManifestTargets,
};
use crate::{ResolvedPackage, Result};

#[derive(Clone)]
pub struct AdditionalManifestFile {
    /// The file path relative to the package path
    pub path: PathBuf,
    /// The base name of the file path
    pub basename: String,
    /// The current content of the file
    pub content: String,
    /// The version regex to use to match and replace version content
    pub version_regex: Regex,
}

impl Default for AdditionalManifestFile {
    fn default() -> Self {
        Self {
            path: "".into(),
            basename: "".into(),
            content: "".into(),
            version_regex: GENERIC_VERSION_REGEX.clone(),
        }
    }
}

impl Serialize for AdditionalManifestFile {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("AdditionalManifestFile", 3)?;
        s.serialize_field("path", &self.path)?;
        s.serialize_field("basename", &self.basename)?;
        s.serialize_field("version_regex", &self.version_regex.as_str())?;
        s.end()
    }
}

impl std::fmt::Debug for AdditionalManifestFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdditionalManifestFile")
            .field("path", &self.path)
            .field("basename", &self.basename)
            .field("version_regex", &self.version_regex)
            .finish()
    }
}

#[derive(Debug)]
pub struct ManifestTarget {
    /// The file path relative to the package path
    pub path: PathBuf,
    /// The base name of the file path
    pub basename: String,
}

#[derive(Default, Clone)]
pub struct ManifestFile {
    /// The file path relative to the package path
    pub path: PathBuf,
    /// The base name of the file path
    pub basename: String,
    /// The current content of the file
    pub content: String,
}

impl Serialize for ManifestFile {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("ManifestFile", 2)?;
        s.serialize_field("path", &self.path)?;
        s.serialize_field("basename", &self.basename)?;
        s.end()
    }
}

impl std::fmt::Debug for ManifestFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManifestFile")
            .field("path", &self.path)
            .field("basename", &self.basename)
            .finish()
    }
}

impl From<AdditionalManifestFile> for ManifestFile {
    fn from(value: AdditionalManifestFile) -> Self {
        Self {
            path: value.path,
            basename: value.basename,
            content: value.content,
        }
    }
}

impl From<&AdditionalManifestFile> for ManifestFile {
    fn from(value: &AdditionalManifestFile) -> Self {
        Self {
            path: value.path.clone(),
            basename: value.basename.clone(),
            content: value.content.clone(),
        }
    }
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
    /// Path to the workspace root directory for this package relative to the repository root
    // pub workspace_root: String,
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
            // workspace_root: pkg.workspace_root.clone(),
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
            (ReleaseType::Java, 6),
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
