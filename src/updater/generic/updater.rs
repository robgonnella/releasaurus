use async_trait::async_trait;

use crate::{
    forge::{request::FileChange, traits::FileLoader},
    result::Result,
    updater::{framework::Package, traits::PackageUpdater},
};

/// Generic package updater for projects without specific language support.
pub struct GenericUpdater {}

impl GenericUpdater {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PackageUpdater for GenericUpdater {
    async fn update(
        &self,
        _packages: Vec<Package>,
        _loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        // nothing to do for generic updater
        Ok(None)
    }
}
