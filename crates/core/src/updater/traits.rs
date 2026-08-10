use std::path::Path;

use crate::{
    forge::request::FileChange, packages::manifests::ManifestFile,
    result::Result, updater::manager::ManifestTarget,
};

/// Common trait for updating version files in different language packages.
pub trait FileUpdater {
    /// Generate file changes to update version numbers across all relevant
    /// files for the package's language/framework.
    fn update(&self, file: &ManifestFile) -> Result<Option<FileChange>>;

    /// Every manifest of this release type, in one pass.
    ///
    /// Defaults to updating each file independently, which is correct
    /// whenever a manifest's new content is a function of its own old
    /// content. Override when one file's content depends on another's
    /// *updated* content: PHP's `composer.lock` carries an md5 of the
    /// bumped `composer.json` beside it, so it cannot be derived
    /// file-by-file.
    fn update_all(&self, files: &[ManifestFile]) -> Result<Vec<FileChange>> {
        let mut changes = vec![];

        for file in files.iter() {
            if let Some(change) = self.update(file)? {
                changes.push(change);
            }
        }

        Ok(changes)
    }
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
