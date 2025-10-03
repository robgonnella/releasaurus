use async_trait::async_trait;

use crate::{
    forge::{request::FileChange, traits::FileLoader},
    result::Result,
    updater::framework::Package,
};

#[async_trait]
/// Common interface for updating version files in different language packages.
pub trait PackageUpdater {
    /// Update version files for packages in the repository.
    async fn update(
        &self,
        packages: Vec<Package>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>>;
}
