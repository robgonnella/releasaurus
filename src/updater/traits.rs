use async_trait::async_trait;

use crate::{
    forge::{request::FileChange, traits::FileLoader},
    result::Result,
    updater::framework::Package,
};

/// Common interface for updating version files in different language
/// packages.
#[async_trait]
pub trait PackageUpdater {
    /// Generate file changes to update version numbers across all relevant
    /// files for the package's language/framework.
    async fn update(
        &self,
        packages: Vec<Package>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>>;
}
