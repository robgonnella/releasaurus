use log::*;
use toml_edit::{DocumentMut, value};

use crate::{
    Result,
    forge::request::{FileChange, FileUpdateType},
    updater::{manager::UpdaterPackage, traits::PackageUpdater},
};

/// Handles Cargo.toml file parsing and version updates for Rust packages.
pub struct CargoToml {}

impl CargoToml {
    /// Create Cargo.toml handler for version updates.
    pub fn new() -> Self {
        Self {}
    }

    fn process_dependencies(
        &self,
        doc: &mut DocumentMut,
        package_name: &str,
        next_version: &str,
        kind: &str,
    ) {
        let dep_exists = doc
            .get(kind)
            .and_then(|deps| deps.as_table())
            .and_then(|t| t.get(package_name))
            .is_some();

        let is_version_object = doc
            .get(kind)
            .and_then(|deps| deps.as_table())
            .and_then(|t| t.get(package_name))
            .map(|p| {
                // Check if it's a table with version field or inline table with
                //  version field
                p.as_table()
                    .map(|t| t.contains_key("version"))
                    .unwrap_or(false)
                    || p.as_inline_table()
                        .map(|t| t.contains_key("version"))
                        .unwrap_or(false)
            })
            .unwrap_or(false);

        if dep_exists {
            if is_version_object {
                doc[kind][&package_name]["version"] = value(next_version);
            } else {
                doc[kind][&package_name] = value(next_version);
            }
        }
    }

    fn load_doc(&self, content: &str) -> Result<DocumentMut> {
        let doc = content.parse::<DocumentMut>()?;
        Ok(doc)
    }
}

impl PackageUpdater for CargoToml {
    /// Update version fields in Cargo.toml files for all Rust packages.
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.basename != "Cargo.toml" {
                continue;
            }

            let mut doc = self.load_doc(&manifest.content)?;

            if doc.get("workspace").is_some() {
                debug!("skipping cargo workspace file");
                continue;
            }

            let next_version = package.next_version.semver.to_string();

            info!(
                "setting version for {} to {next_version}",
                package.package_name
            );

            doc["package"]["version"] = value(&next_version);

            let other_pkgs = workspace_packages
                .iter()
                .filter(|p| p.package_name != package.package_name)
                .cloned()
                .collect::<Vec<UpdaterPackage>>();

            // loop other packages to check if they current manifest deps
            for wkspc_pkg in other_pkgs.iter() {
                let next_version = wkspc_pkg.next_version.semver.to_string();

                self.process_dependencies(
                    &mut doc,
                    &wkspc_pkg.package_name,
                    &next_version,
                    "dependencies",
                );

                self.process_dependencies(
                    &mut doc,
                    &wkspc_pkg.package_name,
                    &next_version,
                    "dev-dependencies",
                );

                self.process_dependencies(
                    &mut doc,
                    &wkspc_pkg.package_name,
                    &next_version,
                    "build-dependencies",
                );
            }

            file_changes.push(FileChange {
                path: manifest.path.clone(),
                content: doc.to_string(),
                update_type: FileUpdateType::Replace,
            });
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::{
        analyzer::release::Tag,
        config::release_type::ReleaseType,
        updater::{
            dispatch::Updater,
            manager::{ManifestFile, UpdaterPackage},
        },
    };

    #[test]
    fn updates_package_version() {
        let cargo_toml = CargoToml::new();
        let content = r#"[package]
name = "my-package"
version = "1.0.0"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "Cargo.toml".to_string(),
            basename: "Cargo.toml".to_string(),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
        };

        let result = cargo_toml.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
    }

    #[test]
    fn updates_workspace_dependency_with_simple_version() {
        let cargo_toml = CargoToml::new();
        let content = r#"[package]
name = "package-a"
version = "1.0.0"

[dependencies]
package-b = "1.0.0"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "packages/a/Cargo.toml".to_string(),
            basename: "Cargo.toml".to_string(),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
        };

        let result = cargo_toml
            .update(&package_a, &[package_a.clone(), package_b])
            .unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("package-b = \"3.0.0\""));
    }

    #[test]
    fn updates_workspace_dependency_with_version_object() {
        let cargo_toml = CargoToml::new();
        let content = r#"[package]
name = "package-a"
version = "1.0.0"

[dependencies]
package-b = { version = "1.0.0", features = ["serde"] }
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "packages/a/Cargo.toml".to_string(),
            basename: "Cargo.toml".to_string(),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
        };

        let result = cargo_toml
            .update(&package_a, &[package_a.clone(), package_b])
            .unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = \"3.0.0\""));
        assert!(updated.contains("features = [\"serde\"]"));
    }

    #[test]
    fn updates_dev_dependencies() {
        let cargo_toml = CargoToml::new();
        let content = r#"[package]
name = "package-a"
version = "1.0.0"

[dev-dependencies]
package-b = "1.0.0"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "packages/a/Cargo.toml".to_string(),
            basename: "Cargo.toml".to_string(),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
        };

        let result = cargo_toml
            .update(&package_a, &[package_a.clone(), package_b])
            .unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("package-b = \"3.0.0\""));
    }

    #[test]
    fn updates_build_dependencies() {
        let cargo_toml = CargoToml::new();
        let content = r#"[package]
name = "package-a"
version = "1.0.0"

[build-dependencies]
package-b = "1.0.0"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "packages/a/Cargo.toml".to_string(),
            basename: "Cargo.toml".to_string(),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
        };

        let result = cargo_toml
            .update(&package_a, &[package_a.clone(), package_b])
            .unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("package-b = \"3.0.0\""));
    }

    #[test]
    fn skips_workspace_cargo_toml() {
        let cargo_toml = CargoToml::new();
        let content = r#"[workspace]
members = ["packages/*"]
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "Cargo.toml".to_string(),
            basename: "Cargo.toml".to_string(),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
        };

        let result = cargo_toml.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn preserves_other_fields() {
        let cargo_toml = CargoToml::new();
        let content = r#"[package]
name = "my-package"
version = "1.0.0"
edition = "2021"
authors = ["Test Author"]

[dependencies]
serde = "1.0"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "Cargo.toml".to_string(),
            basename: "Cargo.toml".to_string(),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
        };

        let result = cargo_toml.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(updated.contains("edition = \"2021\""));
        assert!(updated.contains("authors = [\"Test Author\"]"));
        assert!(updated.contains("serde = \"1.0\""));
    }

    #[test]
    fn process_package_handles_multiple_cargo_toml_files() {
        let cargo_toml = CargoToml::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            path: "packages/a/Cargo.toml".to_string(),
            basename: "Cargo.toml".to_string(),
            content: "[package]\nname = \"package-a\"\nversion = \"1.0.0\"\n"
                .to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            path: "packages/b/Cargo.toml".to_string(),
            basename: "Cargo.toml".to_string(),
            content: "[package]\nname = \"package-b\"\nversion = \"1.0.0\"\n"
                .to_string(),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
        };

        let result = cargo_toml.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[test]
    fn process_package_returns_none_when_no_cargo_toml_files() {
        let cargo_toml = CargoToml::new();
        let manifest = ManifestFile {
            is_workspace: false,
            path: "Cargo.lock".to_string(),
            basename: "Cargo.lock".to_string(),
            content: "version = 3\n".to_string(),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
        };

        let result = cargo_toml.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }
}
