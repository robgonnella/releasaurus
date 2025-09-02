use color_eyre::eyre::Result;
use log::*;
use std::path::Path;

use crate::updater::framework::Package;
use crate::updater::traits::PackageUpdater;

// For use later
// let is_monorepo = manifest_content.contains("\"workspaces\":")
//     || path.join("lerna.json").exists()
//     || manifest_content.contains("\"nx\":");

// let monorepo_root = if is_monorepo {
//     Some(path.to_path_buf())
// } else {
//     None
// };

// // Detect package manager
// let package_manager = if path.join("pnpm-lock.yaml").exists() {
//     "pnpm".to_string()
// } else if path.join("yarn.lock").exists() {
//     "yarn".to_string()
// } else {
//     "npm".to_string()
// };

// let metadata = NodeMetadata {
//     is_monorepo,
//     monorepo_root,
//     package_manager,
// };

/// Node.js package updater supporting npm, yarn, and pnpm
pub struct NodeUpdater {}

impl NodeUpdater {
    pub fn new() -> Self {
        Self {}
    }
}

impl PackageUpdater for NodeUpdater {
    fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()> {
        info!(
            "Found {} node packages in {}",
            packages.len(),
            root_path.display(),
        );
        warn!("Node.js package updater is not implemented yet");

        Ok(())
    }
}
