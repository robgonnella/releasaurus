//! Python updater for handling Python projects with various build systems and
//! package managers
use color_eyre::eyre::Result;
use log::*;
use std::path::{Path, PathBuf};

use crate::updater::framework::Package;
use crate::updater::traits::PackageUpdater;

/// Python updater - handles various Python packaging formats and build systems
pub struct PythonUpdater {
    /// Root directory of the repository
    root_path: PathBuf,
}

impl PythonUpdater {
    /// Create a new Python updater
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        Self {
            root_path: root_path.as_ref().to_path_buf(),
        }
    }
}

impl PackageUpdater for PythonUpdater {
    fn update(&self, packages: Vec<Package>) -> Result<()> {
        info!(
            "Found {} python packages in {}",
            packages.len(),
            self.root_path.display(),
        );
        warn!("Python package updater is not implemented yet");

        Ok(())
    }
}
