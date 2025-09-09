//! Cargo updater for handling rust projects
use color_eyre::eyre::Result;
use log::*;
use std::path::{Path, PathBuf};

use crate::updater::framework::Package;
use crate::updater::traits::PackageUpdater;

pub struct CargoUpdater {
    /// Root directory of the repository
    root_path: PathBuf,
}

impl CargoUpdater {
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        Self {
            root_path: root_path.as_ref().to_path_buf(),
        }
    }
}

impl PackageUpdater for CargoUpdater {
    fn update(&self, packages: Vec<Package>) -> Result<()> {
        info!(
            "Found {} rust packages in {}",
            packages.len(),
            self.root_path.display(),
        );
        warn!("Rust package updater is not implemented yet");

        Ok(())
    }
}
