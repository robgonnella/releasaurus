use toml_edit::{DocumentMut, value};

use crate::{
    Result,
    forge::request::{FileChange, FileUpdateType},
    updater::manager::UpdaterPackage,
};

/// Handles Cargo.lock file parsing and version synchronization for Rust
/// workspace dependencies.
pub struct CargoLock {}

impl CargoLock {
    /// Create Cargo.lock handler for lockfile version updates.
    pub fn new() -> Self {
        Self {}
    }

    pub fn process_package(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.basename != "Cargo.lock" {
                continue;
            }

            let mut lock_doc = self.load_doc(&manifest.content)?;

            if let Some(doc_packages) =
                lock_doc["package"].as_array_of_tables_mut()
            {
                if let Some(main_pkg) = doc_packages.iter_mut().find(|p| {
                    let doc_package_name = p
                        .get("name")
                        .and_then(|item| item.as_str())
                        .unwrap_or("");
                    doc_package_name == package.package_name
                }) {
                    main_pkg["version"] =
                        value(package.next_version.semver.to_string());
                }

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
                    }
                }

                file_changes.push(FileChange {
                    path: manifest.path.clone(),
                    content: lock_doc.to_string(),
                    update_type: FileUpdateType::Replace,
                });
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
        analyzer::release::Tag,
        config::release_type::ReleaseType,
        updater::manager::{ManifestFile, UpdaterPackage},
    };

    #[test]
    fn updates_workspace_package_version() {
        let cargo_lock = CargoLock::new();
        let content = r#"version = 3

[[package]]
name = "my-package"
version = "1.0.0"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "Cargo.lock".to_string(),
            basename: "Cargo.lock".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Rust,
        };

        let result = cargo_lock
            .process_package(&package, slice::from_ref(&package))
            .unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
    }

    #[test]
    fn updates_multiple_workspace_packages() {
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
            path: "Cargo.lock".to_string(),
            basename: "Cargo.lock".to_string(),
            content: content.to_string(),
        };
        let package_a = UpdaterPackage {
            package_name: "package-a".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Rust,
        };
        let package_b = UpdaterPackage {
            package_name: "package-b".to_string(),
            manifest_files: vec![],
            next_version: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::parse("3.0.0").unwrap(),
                sha: "def".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Rust,
        };

        let result = cargo_lock
            .process_package(&package_a, &[package_a.clone(), package_b])
            .unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(updated.contains("version = \"3.0.0\""));
    }

    #[test]
    fn preserves_non_workspace_packages() {
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
            path: "Cargo.lock".to_string(),
            basename: "Cargo.lock".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Rust,
        };

        let result = cargo_lock
            .process_package(&package, slice::from_ref(&package))
            .unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(updated.contains("version = \"5.0.0\""));
    }

    #[test]
    fn preserves_other_fields() {
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
            path: "Cargo.lock".to_string(),
            basename: "Cargo.lock".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Rust,
        };

        let result = cargo_lock
            .process_package(&package, slice::from_ref(&package))
            .unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(updated.contains("dependencies = ["));
        assert!(updated.contains(
            "source = \"registry+https://github.com/rust-lang/crates.io-index\""
        ));
        assert!(updated.contains("checksum = \"abc123\""));
    }

    #[test]
    fn returns_none_when_cargo_lock_has_no_packages() {
        let cargo_lock = CargoLock::new();
        let content = r#"version = 3
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "Cargo.lock".to_string(),
            basename: "Cargo.lock".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Rust,
        };

        let result = cargo_lock
            .process_package(&package, slice::from_ref(&package))
            .unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn process_package_handles_multiple_cargo_lock_files() {
        let cargo_lock = CargoLock::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            path: "workspace/a/Cargo.lock".to_string(),
            basename: "Cargo.lock".to_string(),
            content: "version = 3\n\n[[package]]\nname = \"package-a\"\nversion = \"1.0.0\"\n".to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            path: "workspace/b/Cargo.lock".to_string(),
            basename: "Cargo.lock".to_string(),
            content: "version = 3\n\n[[package]]\nname = \"package-a\"\nversion = \"1.0.0\"\n".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "package-a".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Rust,
        };

        let result = cargo_lock
            .process_package(&package, slice::from_ref(&package))
            .unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[test]
    fn process_package_returns_none_when_no_cargo_lock_files() {
        let cargo_lock = CargoLock::new();
        let manifest = ManifestFile {
            is_workspace: false,
            path: "Cargo.toml".to_string(),
            basename: "Cargo.toml".to_string(),
            content: "[package]\nname = \"my-package\"\nversion = \"1.0.0\"\n"
                .to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Rust,
        };

        let result = cargo_lock.process_package(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn updates_main_package_version_in_cargo_lock() {
        let cargo_lock = CargoLock::new();
        let content = r#"version = 3

[[package]]
name = "main-package"
version = "1.0.0"

[[package]]
name = "workspace-package"
version = "2.0.0"

[[package]]
name = "external-crate"
version = "5.0.0"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "Cargo.lock".to_string(),
            basename: "Cargo.lock".to_string(),
            content: content.to_string(),
        };
        let main_package = UpdaterPackage {
            package_name: "main-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::parse("3.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Rust,
        };
        let workspace_package = UpdaterPackage {
            package_name: "workspace-package".to_string(),
            manifest_files: vec![],
            next_version: Tag {
                name: "v4.0.0".into(),
                semver: semver::Version::parse("4.0.0").unwrap(),
                sha: "def".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Rust,
        };

        let result = cargo_lock
            .process_package(
                &main_package,
                &[main_package.clone(), workspace_package],
            )
            .unwrap();

        let updated = result.unwrap()[0].content.clone();
        // Main package should be updated to 3.0.0
        assert!(updated.contains("name = \"main-package\""));
        assert!(updated.contains("version = \"3.0.0\""));
        // Workspace package should be updated to 4.0.0
        assert!(updated.contains("name = \"workspace-package\""));
        assert!(updated.contains("version = \"4.0.0\""));
        // External package should remain unchanged
        assert!(updated.contains("version = \"5.0.0\""));
    }
}
