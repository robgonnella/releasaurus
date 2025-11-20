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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_helpers::create_test_tag,
        updater::framework::{Framework, ManifestFile, UpdaterPackage},
    };

    #[tokio::test]
    async fn processes_rust_project() {
        let updater = RustUpdater::new();
        let content = r#"[package]
name = "my-package"
version = "1.0.0"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "Cargo.toml".to_string(),
            file_basename: "Cargo.toml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Rust,
        };

        let result = updater.update(&package, vec![]).await.unwrap();

        assert!(result.is_some());
        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[tokio::test]
    async fn returns_none_when_no_rust_files() {
        let updater = RustUpdater::new();
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "package.json".to_string(),
            file_basename: "package.json".to_string(),
            content: r#"{"version":"1.0.0"}"#.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Rust,
        };

        let result = updater.update(&package, vec![]).await.unwrap();

        assert!(result.is_none());
    }
}
