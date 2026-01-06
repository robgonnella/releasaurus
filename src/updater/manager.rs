//! Framework and package management for multi-language support.
use log::*;
use regex::Regex;
use std::borrow::Cow;
use std::path::Path;
use std::rc::Rc;

use crate::Result;
use crate::analyzer::release::Tag;
use crate::cli::types::ReleasablePackage;
use crate::config::package::PackageConfig;
use crate::config::release_type::ReleaseType;
use crate::file_loader::FileLoader;
use crate::forge::request::FileChange;
use crate::path_helpers::package_path;
use crate::updater::generic::updater::GENERIC_VERSION_REGEX;
use crate::updater::{
    dispatch::Updater, generic::updater::GenericUpdater,
    java::manifests::JavaManifests, node::manifests::NodeManifests,
    php::manifests::PhpManifests, python::manifests::PythonManifests,
    ruby::manifests::RubyManifests, rust::manifests::RustManifests,
    traits::ManifestTargets,
};

#[derive(Clone)]
pub struct AdditionalManifestFile {
    /// The file path relative to the package path
    pub path: String,
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
    pub path: String,
    /// The base name of the file path
    pub basename: String,
}

#[derive(Default, Clone)]
pub struct ManifestFile {
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
        pkg: &PackageConfig,
        file_loader: &F,
        branch: Option<String>,
    ) -> Result<Option<Vec<ManifestFile>>> {
        let targets = Self::release_type_manifest_targets(pkg);

        if targets.is_empty() {
            return Ok(None);
        }

        let mut manifests = vec![];

        for target in targets {
            debug!("Loading manifest target: {}", target.path);
            if let Some(content) = file_loader
                .load_file(branch.clone(), target.path.clone())
                .await?
            {
                info!("Loaded manifest: {}", target.path);
                manifests.push(ManifestFile {
                    path: target.path,
                    basename: target.basename,
                    content,
                });
            } else {
                debug!("Manifest not found: {}", target.path);
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
        pkg: &PackageConfig,
        file_loader: &F,
        branch: Option<String>,
    ) -> Result<Option<Vec<AdditionalManifestFile>>> {
        if pkg.compiled_additional_manifests.is_empty() {
            return Ok(None);
        }

        let mut manifests = vec![];

        for compiled in &pkg.compiled_additional_manifests {
            let file_path = package_path(pkg, Some(&compiled.path));
            let basename = Path::new(&compiled.path)
                .file_name()
                .map(|f| f.to_string_lossy())
                .unwrap_or(Cow::Borrowed(&compiled.path))
                .into();

            debug!("Loading additional manifest: {}", file_path);
            if let Some(content) = file_loader
                .load_file(branch.clone(), file_path.clone())
                .await?
            {
                info!("Loaded additional manifest: {}", file_path);

                manifests.push(AdditionalManifestFile {
                    path: file_path,
                    basename,
                    content,
                    version_regex: compiled.version_regex.clone(),
                });
            } else {
                debug!("Additional manifest not found: {}", file_path);
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
            "Package: {}: Found {} other packages for workspace root: {}",
            updater_package.package_name,
            workspace_updater_packages.len(),
            package.workspace_root
        );

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

        Ok(file_changes)
    }

    fn release_type_manifest_targets(
        pkg: &PackageConfig,
    ) -> Vec<ManifestTarget> {
        match pkg.release_type {
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
        let release_type = pkg.release_type;
        let updater = Rc::new(Updater::new(release_type));

        UpdaterPackage {
            package_name: pkg.name.clone(),
            // workspace_root: pkg.workspace_root.clone(),
            manifest_files: pkg
                .manifest_files
                .as_ref()
                .cloned()
                .unwrap_or_default(),
            next_version: pkg.release.tag.clone(),
            updater,
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
            let pkg = create_test_package(Some(release_type));
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
}
