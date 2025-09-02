//! Cargo updater for handling rust projects
use color_eyre::eyre::Result;
use log::*;
use std::path::Path;

use crate::updater::framework::Package;
use crate::updater::traits::PackageUpdater;

// For use later
// let is_workspace = manifest_content.contains("[workspace]");
// let workspace_root = if is_workspace {
//     Some(path.to_path_buf())
// } else {
//     None
// };

// let metadata = RustMetadata {
//     is_workspace,
//     workspace_root,
//     package_manager: "cargo".to_string(),
// };

pub struct CargoUpdater {}

impl CargoUpdater {
    pub fn new() -> Self {
        Self {}
    }
}

impl PackageUpdater for CargoUpdater {
    fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()> {
        info!(
            "Found {} rust packages in {}",
            packages.len(),
            root_path.display(),
        );
        warn!("Rust package updater is not implemented yet");

        Ok(())
    }
}
