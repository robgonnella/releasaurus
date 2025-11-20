use toml_edit::{DocumentMut, value};

use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::UpdaterPackage,
};

/// Handles Cargo.lock file parsing and version synchronization for Rust
/// workspace dependencies.
pub struct CargoLock {}

impl CargoLock {
    /// Create Cargo.lock handler for lockfile version updates.
    pub fn new() -> Self {
        Self {}
    }

    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename != "Cargo.lock" {
                continue;
            }

            let mut lock_doc = self.load_doc(&manifest.content)?;

            if let Some(doc_packages) =
                lock_doc["package"].as_array_of_tables_mut()
            {
                let mut updated = false;
                // Update all packages in this workspace
                for pkg in workspace_packages.iter() {
                    if let Some(found) = doc_packages.iter_mut().find(|p| {
                        let doc_package_name = p
                            .get("name")
                            .and_then(|item| item.as_str())
                            .unwrap_or("");
                        doc_package_name == pkg.package_name
                    }) {
                        found["version"] =
                            value(pkg.next_version.semver.to_string());
                        updated = true;
                    }
                }

                if updated {
                    file_changes.push(FileChange {
                        path: manifest.file_path.clone(),
                        content: lock_doc.to_string(),
                        update_type: FileUpdateType::Replace,
                    });
                }
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    fn load_doc(&self, content: &str) -> Result<DocumentMut> {
        let doc = content.parse::<DocumentMut>()?;
        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use std::slice;

    use super::*;
    use crate::{
        test_helpers::create_test_tag,
        updater::framework::{Framework, ManifestFile, UpdaterPackage},
    };

    #[tokio::test]
    async fn updates_workspace_package_version() {
        let cargo_lock = CargoLock::new();
        let content = r#"version = 3

[[package]]
name = "my-package"
version = "1.0.0"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "Cargo.lock".to_string(),
            file_basename: "Cargo.lock".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Rust,
        };

        let result = cargo_lock
            .process_package(&package, slice::from_ref(&package))
            .await
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
    }

    #[tokio::test]
    async fn updates_multiple_workspace_packages() {
        let cargo_lock = CargoLock::new();
        let content = r#"version = 3

[[package]]
name = "package-a"
version = "1.0.0"

[[package]]
name = "package-b"
version = "1.0.0"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "Cargo.lock".to_string(),
            file_basename: "Cargo.lock".to_string(),
            content: content.to_string(),
        };
        let package_a = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: "packages/a".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Rust,
        };
        let package_b = UpdaterPackage {
            package_name: "package-b".to_string(),
            workspace_root: "packages/b".to_string(),
            manifest_files: vec![],
            next_version: create_test_tag("v3.0.0", "3.0.0", "def"),
            framework: Framework::Rust,
        };

        let result = cargo_lock
            .process_package(&package_a, &[package_a.clone(), package_b])
            .await
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(updated.contains("version = \"3.0.0\""));
    }

    #[tokio::test]
    async fn preserves_non_workspace_packages() {
        let cargo_lock = CargoLock::new();
        let content = r#"version = 3

[[package]]
name = "my-package"
version = "1.0.0"

[[package]]
name = "external-crate"
version = "5.0.0"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "Cargo.lock".to_string(),
            file_basename: "Cargo.lock".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Rust,
        };

        let result = cargo_lock
            .process_package(&package, slice::from_ref(&package))
            .await
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(updated.contains("version = \"5.0.0\""));
    }

    #[tokio::test]
    async fn preserves_other_fields() {
        let cargo_lock = CargoLock::new();
        let content = r#"version = 3

[[package]]
name = "my-package"
version = "1.0.0"
dependencies = [
    "serde",
]

[[package]]
name = "serde"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "abc123"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "Cargo.lock".to_string(),
            file_basename: "Cargo.lock".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Rust,
        };

        let result = cargo_lock
            .process_package(&package, slice::from_ref(&package))
            .await
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(updated.contains("dependencies = ["));
        assert!(updated.contains(
            "source = \"registry+https://github.com/rust-lang/crates.io-index\""
        ));
        assert!(updated.contains("checksum = \"abc123\""));
    }

    #[tokio::test]
    async fn returns_none_when_no_workspace_packages_to_update() {
        let cargo_lock = CargoLock::new();
        let content = r#"version = 3

[[package]]
name = "external-crate"
version = "5.0.0"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "Cargo.lock".to_string(),
            file_basename: "Cargo.lock".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Rust,
        };

        let result = cargo_lock
            .process_package(&package, slice::from_ref(&package))
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn process_package_handles_multiple_cargo_lock_files() {
        let cargo_lock = CargoLock::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            file_path: "workspace/a/Cargo.lock".to_string(),
            file_basename: "Cargo.lock".to_string(),
            content: "version = 3\n\n[[package]]\nname = \"package-a\"\nversion = \"1.0.0\"\n".to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            file_path: "workspace/b/Cargo.lock".to_string(),
            file_basename: "Cargo.lock".to_string(),
            content: "version = 3\n\n[[package]]\nname = \"package-a\"\nversion = \"1.0.0\"\n".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Rust,
        };

        let result = cargo_lock
            .process_package(&package, slice::from_ref(&package))
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[tokio::test]
    async fn process_package_returns_none_when_no_cargo_lock_files() {
        let cargo_lock = CargoLock::new();
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "Cargo.toml".to_string(),
            file_basename: "Cargo.toml".to_string(),
            content: "[package]\nname = \"my-package\"\nversion = \"1.0.0\"\n"
                .to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Rust,
        };

        let result = cargo_lock.process_package(&package, &[]).await.unwrap();

        assert!(result.is_none());
    }
}
