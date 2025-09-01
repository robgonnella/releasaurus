use color_eyre::eyre::Result;
use log::*;
use std::path::{Path, PathBuf};

use crate::updater::framework::Package;
use crate::updater::traits::PackageUpdater;

/// Node.js package updater supporting npm, yarn, and pnpm
pub struct NodeUpdater {
    /// Root path of the repository
    root_path: PathBuf,
}

impl NodeUpdater {
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        Self {
            root_path: root_path.as_ref().to_path_buf(),
        }
    }
}

impl PackageUpdater for NodeUpdater {
    fn update(&self, packages: Vec<Package>) -> Result<()> {
        info!(
            "Found {} node packages in {}",
            packages.len(),
            self.root_path.display(),
        );
        warn!("Node.js package updater is not implemented yet");

        Ok(())
    }
}
