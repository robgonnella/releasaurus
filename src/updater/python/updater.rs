//! Python updater for handling Python projects with various build systems and
//! package managers
use color_eyre::eyre::Result;
use log::*;
use std::path::Path;

use crate::updater::framework::Package;
use crate::updater::traits::PackageUpdater;

// For use later
// // Detect build system
// let build_system = if manifest_content.contains("[tool.poetry]")
// {
//     "poetry".to_string()
// } else if manifest_content.contains("[tool.setuptools]") {
//     "setuptools".to_string()
// } else if manifest_content.contains("[tool.flit]") {
//     "flit".to_string()
// } else {
//     "setuptools".to_string()
// };

// // Detect package manager
// let package_manager = if path.join("poetry.lock").exists() {
//     "poetry".to_string()
// } else if path.join("Pipfile").exists() {
//     "pipenv".to_string()
// } else {
//     "pip".to_string()
// };

// let metadata = PythonMetadata {
//     build_system,
//     package_manager,
//     uses_pyproject: true,
// };

// let metadata = PythonMetadata {
//     build_system: "setuptools".to_string(),
//     package_manager: "pip".to_string(),
//     uses_pyproject: false,
// };

/// Python updater - handles various Python packaging formats and build systems
pub struct PythonUpdater {}

impl PythonUpdater {
    /// Create a new Python updater
    pub fn new() -> Self {
        Self {}
    }
}

impl PackageUpdater for PythonUpdater {
    fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()> {
        info!(
            "Found {} python packages in {}",
            packages.len(),
            root_path.display(),
        );
        warn!("Python package updater is not implemented yet");

        Ok(())
    }
}
