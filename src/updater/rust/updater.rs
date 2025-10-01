//! Cargo updater for handling rust projects
use std::path::Path;

use async_trait::async_trait;
use log::*;

use crate::{
    forge::{request::FileChange, traits::FileLoader},
    result::Result,
    updater::{
        framework::{Framework, Package},
        rust::{cargo_lock::CargoLock, cargo_toml::CargoToml},
        traits::PackageUpdater,
    },
};

/// Rust package updater for Cargo projects.
pub struct RustUpdater {
    cargo_toml: CargoToml,
    cargo_lock: CargoLock,
}

impl RustUpdater {
    pub fn new() -> Self {
        Self {
            cargo_toml: CargoToml::new(),
            cargo_lock: CargoLock::new(),
        }
    }
}

#[async_trait]
impl PackageUpdater for RustUpdater {
    async fn update(
        &self,
        packages: Vec<Package>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        let rust_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Rust))
            .collect::<Vec<Package>>();

        info!("Found {} rust packages", rust_packages.len());

        let root_path = Path::new(".");

        let packages_with_names = self
            .cargo_toml
            .get_packages_with_names(rust_packages, loader)
            .await;

        if self.cargo_toml.is_workspace(root_path, loader).await?
            && let Some(change) = self
                .cargo_lock
                .process_workspace_lockfile(
                    root_path,
                    &packages_with_names,
                    loader,
                )
                .await?
        {
            file_changes.push(change);
        }

        if let Some(changes) = self
            .cargo_toml
            .process_packages(&packages_with_names, loader)
            .await?
        {
            file_changes.extend(changes);
        }

        if let Some(changes) = self
            .cargo_lock
            .process_packages(&packages_with_names, loader)
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
