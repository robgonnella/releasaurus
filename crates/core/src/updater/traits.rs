use std::path::Path;

use crate::{
    forge::request::FileChange,
    result::Result,
    updater::manager::{ManifestTarget, UpdaterPackage},
};

/// Common trait for updating version files in different language packages.
pub trait PackageUpdater {
    /// Generate file changes to update version numbers across all relevant
    /// files for the package's language/framework.
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>>;
}

/// Resolves the set of manifest file paths to load for a given
/// package and release type.
pub trait ManifestTargets {
    /// Returns the manifest file targets to load from the forge
    /// for this package.
    fn manifest_targets(
        pkg_name: &str,
        workspace_path: &Path,
        pkg_path: &Path,
    ) -> Vec<ManifestTarget>;
}
