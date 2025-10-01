//! Python updater for handling Python projects with various build systems and
//! package managers
use async_trait::async_trait;
use log::*;

use crate::{
    forge::{request::FileChange, traits::FileLoader},
    result::Result,
    updater::{
        framework::{Framework, Package},
        python::{pyproject::PyProject, setupcfg::SetupCfg, setuppy::SetupPy},
        traits::PackageUpdater,
    },
};

/// Python updater - handles various Python packaging formats and build systems
/// Python package updater supporting pyproject.toml, setup.py, and setup.cfg.
pub struct PythonUpdater {
    pyproject: PyProject,
    setuppy: SetupPy,
    setupcfg: SetupCfg,
}

impl PythonUpdater {
    /// Create a new Python updater
    pub fn new() -> Self {
        Self {
            pyproject: PyProject::new(),
            setuppy: SetupPy::new(),
            setupcfg: SetupCfg::new(),
        }
    }
}

#[async_trait]
impl PackageUpdater for PythonUpdater {
    async fn update(
        &self,
        packages: Vec<Package>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let python_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Python))
            .collect::<Vec<Package>>();

        info!("Found {} python packages", python_packages.len());

        let mut file_changes: Vec<FileChange> = vec![];
        if let Some(changes) = self
            .pyproject
            .process_packages(&python_packages, loader)
            .await?
        {
            file_changes.extend(changes);
        }

        if let Some(changes) = self
            .setuppy
            .process_packages(&python_packages, loader)
            .await?
        {
            file_changes.extend(changes);
        }

        if let Some(changes) = self
            .setupcfg
            .process_packages(&python_packages, loader)
            .await?
        {
            file_changes.extend(changes);
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}
