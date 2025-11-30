use crate::{
    Result,
    config::package::PackageConfig,
    forge::request::FileChange,
    updater::manager::{ManifestTarget, UpdaterPackage},
};

/// Common trait for updating version files in different language packages.
pub trait PackageUpdater {
    /// Generate file changes to update version numbers across all relevant
    /// files for the package's language/framework.
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>>;
}

/// Common trait for loading targeted manifests for each package release_type
pub trait ManifestTargets {
    /// Loads a list of manifest targets to look for in the forge
    fn manifest_targets(pkg: &PackageConfig) -> Vec<ManifestTarget>;
}
