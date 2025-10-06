//! Cargo updater for handling rust projects
use std::path::Path;

use async_trait::async_trait;
use log::*;

use crate::{
    forge::{request::FileChange, traits::FileLoader},
    result::Result,
    updater::{
        framework::{Framework, UpdaterPackage},
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
        packages: Vec<UpdaterPackage>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        let rust_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Rust))
            .collect::<Vec<UpdaterPackage>>();

        info!("Found {} rust packages", rust_packages.len());

        if rust_packages.is_empty() {
            return Ok(None);
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::release::Tag;
    use crate::forge::traits::MockFileLoader;
    use semver::Version as SemVer;

    fn create_test_package(
        name: &str,
        path: &str,
        next_version: &str,
    ) -> UpdaterPackage {
        UpdaterPackage {
            name: name.to_string(),
            path: path.to_string(),
            framework: Framework::Rust,
            next_version: Tag {
                sha: "test-sha".to_string(),
                name: format!("v{}", next_version),
                semver: SemVer::parse(next_version).unwrap(),
            },
        }
    }

    #[tokio::test]
    async fn test_update_single_package() {
        let updater = RustUpdater::new();
        let package =
            create_test_package("test-crate", "packages/test", "2.0.0");

        let cargo_toml = r#"[package]
name = "test-crate"
version = "1.0.0"
edition = "2021"

[dependencies]
serde = "1.0"
"#;

        let mut mock_loader = MockFileLoader::new();

        // Check for workspace at root
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./Cargo.toml"))
            .times(1)
            .returning(|_| Ok(None));

        // Get package names
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_toml.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Process packages
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_toml.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Check for Cargo.lock
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/Cargo.toml");
        assert!(changes[0].content.contains("version = \"2.0.0\""));
        assert!(changes[0].content.contains("name = \"test-crate\""));
    }

    #[tokio::test]
    async fn test_update_package_with_dependencies() {
        let updater = RustUpdater::new();
        let packages = vec![
            create_test_package("crate-a", "packages/a", "2.0.0"),
            create_test_package("crate-b", "packages/b", "3.0.0"),
        ];

        let cargo_a = r#"[package]
name = "crate-a"
version = "1.0.0"

[dependencies]
crate-b = "1.0.0"
serde = "1.0"
"#;

        let cargo_b = r#"[package]
name = "crate-b"
version = "1.0.0"
"#;

        let mut mock_loader = MockFileLoader::new();

        // Workspace check
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./Cargo.toml"))
            .times(1)
            .returning(|_| Ok(None));

        // Get package names - crate-a
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_a.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Get package names - crate-b
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_b.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Process packages - crate-a
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_a.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Process packages - crate-b
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_b.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Cargo.lock checks
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);

        // Check crate-a
        let change_a = changes
            .iter()
            .find(|c| c.path == "packages/a/Cargo.toml")
            .unwrap();
        assert!(change_a.content.contains("version = \"2.0.0\""));
        assert!(change_a.content.contains("crate-b = \"3.0.0\""));

        // Check crate-b
        let change_b = changes
            .iter()
            .find(|c| c.path == "packages/b/Cargo.toml")
            .unwrap();
        assert!(change_b.content.contains("version = \"3.0.0\""));
    }

    #[tokio::test]
    async fn test_update_package_with_dev_dependencies() {
        let updater = RustUpdater::new();
        let packages = vec![
            create_test_package("crate-a", "packages/a", "2.0.0"),
            create_test_package("crate-b", "packages/b", "3.0.0"),
        ];

        let cargo_a = r#"[package]
name = "crate-a"
version = "1.0.0"

[dev-dependencies]
crate-b = "1.0.0"
"#;

        let cargo_b = r#"[package]
name = "crate-b"
version = "1.0.0"
"#;

        let mut mock_loader = MockFileLoader::new();

        // Workspace check
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./Cargo.toml"))
            .times(1)
            .returning(|_| Ok(None));

        // Get package names
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_a.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_b.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Process packages
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_a.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_b.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Cargo.lock checks
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();

        let change_a = changes
            .iter()
            .find(|c| c.path == "packages/a/Cargo.toml")
            .unwrap();
        assert!(change_a.content.contains("crate-b = \"3.0.0\""));
    }

    #[tokio::test]
    async fn test_update_package_with_version_object() {
        let updater = RustUpdater::new();
        let packages = vec![
            create_test_package("crate-a", "packages/a", "2.0.0"),
            create_test_package("crate-b", "packages/b", "3.0.0"),
        ];

        let cargo_a = r#"[package]
name = "crate-a"
version = "1.0.0"

[dependencies]
crate-b = { version = "1.0.0", features = ["extra"] }
"#;

        let cargo_b = r#"[package]
name = "crate-b"
version = "1.0.0"
"#;

        let mut mock_loader = MockFileLoader::new();

        // Workspace check
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./Cargo.toml"))
            .times(1)
            .returning(|_| Ok(None));

        // Get package names
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_a.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_b.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Process packages
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_a.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_b.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Cargo.lock checks
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();

        let change_a = changes
            .iter()
            .find(|c| c.path == "packages/a/Cargo.toml")
            .unwrap();
        assert!(change_a.content.contains("version = \"3.0.0\""));
        assert!(change_a.content.contains("features = [\"extra\"]"));
    }

    #[tokio::test]
    async fn test_update_workspace_cargo_lock() {
        let updater = RustUpdater::new();
        let packages = vec![
            create_test_package("crate-a", "packages/a", "2.0.0"),
            create_test_package("crate-b", "packages/b", "3.0.0"),
        ];

        let workspace_toml = r#"[workspace]
members = ["packages/a", "packages/b"]
"#;

        let cargo_a = r#"[package]
name = "crate-a"
version = "1.0.0"
"#;

        let cargo_b = r#"[package]
name = "crate-b"
version = "1.0.0"
"#;

        let cargo_lock = r#"version = 3

[[package]]
name = "crate-a"
version = "1.0.0"

[[package]]
name = "crate-b"
version = "1.0.0"
"#;

        let mut mock_loader = MockFileLoader::new();

        // Workspace check
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./Cargo.toml"))
            .times(1)
            .returning({
                let content = workspace_toml.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Workspace Cargo.lock
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./Cargo.lock"))
            .times(1)
            .returning({
                let content = cargo_lock.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Get package names
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_a.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_b.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Process packages
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_a.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_b.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Individual Cargo.lock checks
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 3); // workspace lock + 2 Cargo.toml files

        // Check workspace Cargo.lock was updated
        let lock_change =
            changes.iter().find(|c| c.path == "./Cargo.lock").unwrap();
        assert!(lock_change.content.contains("version = \"2.0.0\""));
        assert!(lock_change.content.contains("version = \"3.0.0\""));
    }

    #[tokio::test]
    async fn test_update_individual_cargo_lock() {
        let updater = RustUpdater::new();
        let packages = vec![
            create_test_package("crate-a", "packages/a", "2.0.0"),
            create_test_package("crate-b", "packages/b", "3.0.0"),
        ];

        let cargo_a = r#"[package]
name = "crate-a"
version = "1.0.0"

[dependencies]
crate-b = "1.0.0"
"#;

        let cargo_b = r#"[package]
name = "crate-b"
version = "1.0.0"
"#;

        let cargo_lock_a = r#"version = 3

[[package]]
name = "crate-a"
version = "1.0.0"

[[package]]
name = "crate-b"
version = "1.0.0"
"#;

        let mut mock_loader = MockFileLoader::new();

        // Workspace check
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./Cargo.toml"))
            .times(1)
            .returning(|_| Ok(None));

        // Get package names
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_a.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_b.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Process packages
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_a.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_b.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Cargo.lock for crate-a exists
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.lock"))
            .times(1)
            .returning({
                let content = cargo_lock_a.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Cargo.lock for crate-b doesn't exist
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 3); // 2 Cargo.toml + 1 Cargo.lock

        // Check Cargo.lock was updated
        let lock_change = changes
            .iter()
            .find(|c| c.path == "packages/a/Cargo.lock")
            .unwrap();
        assert!(lock_change.content.contains("version = \"2.0.0\""));
        assert!(lock_change.content.contains("version = \"3.0.0\""));
    }

    #[tokio::test]
    async fn test_update_filters_rust_packages() {
        let updater = RustUpdater::new();

        let packages = vec![
            create_test_package("rust-crate", "packages/rust", "2.0.0"),
            UpdaterPackage {
                name: "python-package".to_string(),
                path: "packages/python".to_string(),
                framework: Framework::Python,
                next_version: Tag {
                    sha: "test-sha".to_string(),
                    name: "v1.0.0".to_string(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                },
            },
        ];

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        // Should return None when no Rust files are found
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_no_files_found() {
        let updater = RustUpdater::new();
        let package =
            create_test_package("test-crate", "packages/test", "2.0.0");

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_skips_workspace_cargo_toml() {
        let updater = RustUpdater::new();
        let package = create_test_package("workspace-root", ".", "2.0.0");

        let workspace_toml = r#"[workspace]
members = ["packages/a", "packages/b"]
"#;

        let mut mock_loader = MockFileLoader::new();

        // Workspace check at root
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./Cargo.toml"))
            .times(1)
            .returning({
                let content = workspace_toml.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // No workspace Cargo.lock
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        // Get package names
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./Cargo.toml"))
            .times(1)
            .returning({
                let content = workspace_toml.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Process packages - should skip workspace file
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./Cargo.toml"))
            .times(1)
            .returning({
                let content = workspace_toml.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Check for Cargo.lock
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        // Should return None because workspace Cargo.toml is skipped
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_with_build_dependencies() {
        let updater = RustUpdater::new();
        let packages = vec![
            create_test_package("crate-a", "packages/a", "2.0.0"),
            create_test_package("crate-b", "packages/b", "3.0.0"),
        ];

        let cargo_a = r#"[package]
name = "crate-a"
version = "1.0.0"

[build-dependencies]
crate-b = "1.0.0"
"#;

        let cargo_b = r#"[package]
name = "crate-b"
version = "1.0.0"
"#;

        let mut mock_loader = MockFileLoader::new();

        // Workspace check
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./Cargo.toml"))
            .times(1)
            .returning(|_| Ok(None));

        // Get package names
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_a.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_b.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Process packages
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_a.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.toml"))
            .times(1)
            .returning({
                let content = cargo_b.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Cargo.lock checks
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/Cargo.lock"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();

        let change_a = changes
            .iter()
            .find(|c| c.path == "packages/a/Cargo.toml")
            .unwrap();
        assert!(change_a.content.contains("crate-b = \"3.0.0\""));
    }
}
