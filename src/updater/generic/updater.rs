use std::path::Path;

use crate::{
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

impl PackageUpdater for GenericUpdater {
    fn update(&self, _root_path: &Path, _packages: Vec<Package>) -> Result<()> {
        // nothing to do for generic updater
        Ok(())
    }
}
