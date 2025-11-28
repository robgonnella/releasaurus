use async_trait::async_trait;

use crate::{
    cli::Result,
    forge::request::FileChange,
    updater::{framework::UpdaterPackage, traits::PackageUpdater},
};

/// Generic package updater for projects without specific language support.
pub struct GenericUpdater {}

impl GenericUpdater {
    /// Create generic updater that performs no version file updates.
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PackageUpdater for GenericUpdater {
    async fn update(
        &self,
        _package: &UpdaterPackage,
        _workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        // nothing to do for generic updater
        Ok(None)
    }
}
