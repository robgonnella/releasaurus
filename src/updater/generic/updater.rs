use color_eyre::eyre::Result;

use crate::updater::{framework::Package, traits::PackageUpdater};

pub struct GenericUpdater {}

impl GenericUpdater {
    pub fn new() -> Self {
        Self {}
    }
}

impl PackageUpdater for GenericUpdater {
    fn update(&self, _packages: Vec<Package>) -> Result<()> {
        // nothing to do for generic updater
        Ok(())
    }
}
