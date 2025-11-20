//! Cargo updater for handling rust projects
use async_trait::async_trait;

use crate::{
    forge::request::FileChange,
    result::Result,
    updater::{
        framework::UpdaterPackage,
        rust::{cargo_lock::CargoLock, cargo_toml::CargoToml},
        traits::PackageUpdater,
    },
};

/// Updates Cargo.toml and Cargo.lock files for Rust packages, handling
/// workspace dependencies and version synchronization.
pub struct RustUpdater {
    cargo_toml: CargoToml,
    cargo_lock: CargoLock,
}

impl RustUpdater {
    /// Create Rust updater with Cargo.toml and Cargo.lock handlers.
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
        package: &UpdaterPackage,
        workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        if let Some(changes) = self
            .cargo_toml
            .process_package(package, &workspace_packages)
            .await?
        {
            file_changes.extend(changes);
        }

        if let Some(changes) = self
            .cargo_lock
            .process_package(package, &workspace_packages)
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
