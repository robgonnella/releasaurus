use std::path::Path;

use crate::{result::Result, updater::framework::Package};

/// Common interface for updating version files in different language packages.
pub trait PackageUpdater {
    /// Update version files for packages in the repository.
    fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()>;
}
