use toml_edit::{DocumentMut, TableLike, value};

use crate::{
    forge::request::{FileChange, FileUpdateType},
    packages::manifests::ManifestFile,
    result::Result,
    updater::traits::FileUpdater,
};

/// Dependency tables a package declares its own requirements in.
const DEP_KINDS: [&str; 3] =
    ["dependencies", "dev-dependencies", "build-dependencies"];

/// Handles Cargo.toml file parsing and version updates for Rust packages.
pub struct CargoToml {}

impl Default for CargoToml {
    fn default() -> Self {
        CargoToml::new()
    }
}

impl CargoToml {
    /// Create Cargo.toml handler for version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Writes `next_version` into `name`'s entry in `table`, if that
    /// entry exists and carries a literal version. Reports whether
    /// anything changed.
    fn set_dep_version(
        table: &mut dyn TableLike,
        name: &str,
        next_version: &str,
    ) -> bool {
        let Some(dep) = table.get_mut(name) else {
            return false;
        };

        // Plain `name = "1.2.3"`.
        if dep.is_str() {
            if dep.as_str() == Some(next_version) {
                return false;
            }
            *dep = value(next_version);
            return true;
        }

        // `name = { version = "1.2.3", .. }`. An entry with no version
        // key is either path-only or inheriting via `workspace = true`;
        // both must keep their shape, so replacing the whole value with
        // a bare string would corrupt them.
        let Some(existing) =
            dep.as_table_like_mut().and_then(|t| t.get_mut("version"))
        else {
            return false;
        };

        if existing.as_str() == Some(next_version) {
            return false;
        }

        *existing = value(next_version);
        true
    }

    /// `Item::get_mut` auto-vivifies — it routes through `Index for str`,
    /// which inserts `Item::None` for a missing key. Probing
    /// `[workspace]` for a `dependencies` table that isn't there would
    /// leave `dependencies = {}` behind. `Table::get_mut` and
    /// `TableLike::get_mut` both filter `is_none`, so only those are
    /// safe here.
    fn dep_table_mut<'a>(
        doc: &'a mut DocumentMut,
        path: &[&str],
    ) -> Option<&'a mut dyn TableLike> {
        let mut table: &mut dyn TableLike = doc.as_table_mut();

        for key in path {
            table = table.get_mut(key)?.as_table_like_mut()?;
        }

        Some(table)
    }

    fn load_doc(&self, content: &str) -> Result<DocumentMut> {
        let doc = content.parse::<DocumentMut>()?;
        Ok(doc)
    }
}

impl FileUpdater for CargoToml {
    /// Update version fields in Cargo.toml files for all Rust packages.
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename != "Cargo.toml" {
            return Ok(None);
        }

        let mut doc = self.load_doc(&manifest.content)?;
        let mut changed = false;

        // Two ways there is no `[package] version` to write: a virtual
        // workspace manifest has no `[package]` table, and a root that is
        // not part of this release has no owner. Indexing `[package]` into
        // existence would invent a package carrying nothing but a version.
        if let Some(owner) = manifest.owner.as_ref() {
            let next_version = owner.tag.semver.to_string();

            if let Some(pkg_table) = Self::dep_table_mut(&mut doc, &["package"])
                && pkg_table.get("version").and_then(|v| v.as_str())
                    != Some(next_version.as_str())
            {
                log::info!(
                    "setting version for {} to {next_version}",
                    owner.name
                );

                pkg_table.insert("version", value(&next_version));
                changed = true;
            }
        }

        let next_versions = manifest
            .releasing
            .iter()
            .map(|p| (p.name.clone(), p.tag.semver.to_string()))
            .collect::<Vec<(String, String)>>();

        for kind in DEP_KINDS {
            let Some(table) = Self::dep_table_mut(&mut doc, &[kind]) else {
                continue;
            };

            for (name, next) in next_versions.iter() {
                changed |= Self::set_dep_version(table, name, next);
            }
        }

        // A workspace root declares shared versions once; members
        // referencing them with `workspace = true` inherit the bump from
        // here, so this has to be synced whether or not the root itself
        // is a package.
        if let Some(table) =
            Self::dep_table_mut(&mut doc, &["workspace", "dependencies"])
        {
            for (name, next) in next_versions.iter() {
                changed |= Self::set_dep_version(table, name, next);
            }
        }

        if !changed {
            return Ok(None);
        }

        Ok(Some(FileChange {
            path: manifest.path.to_string_lossy().to_string(),
            content: doc.to_string(),
            update_type: FileUpdateType::Replace,
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        config::release_type::ReleaseType,
        forge::request::Tag,
        packages::manifests::{ManifestFile, ManifestPackage},
    };

    use super::*;

    #[test]
    fn updates_package_version() {
        let cargo_toml = CargoToml::new();
        let content = r#"[package]
name = "my-package"
version = "1.0.0"
"#;

        let manifest = ManifestFile {
            path: Path::new("Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".into(),
            content: content.into(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "my-package".into(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = cargo_toml.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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

        let package_b = ManifestPackage {
            name: "package-b".into(),
            release_type: ReleaseType::Rust,
            tag: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::parse("3.0.0").unwrap(),
                sha: "def".into(),
                ..Tag::default()
            },
        };

        let manifest = ManifestFile {
            path: Path::new("packages/a/Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".to_string(),
            content: content.into(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "package-a".to_string(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![package_b],
        };

        let result = cargo_toml.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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

        let package_b = ManifestPackage {
            name: "package-b".into(),
            release_type: ReleaseType::Rust,
            tag: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::parse("3.0.0").unwrap(),
                sha: "def".into(),
                ..Tag::default()
            },
        };

        let manifest = ManifestFile {
            path: Path::new("packages/a/Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".to_string(),
            content: content.into(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "package-a".to_string(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![package_b],
        };

        let result = cargo_toml.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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

        let package_b = ManifestPackage {
            name: "package-b".into(),
            release_type: ReleaseType::Rust,
            tag: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::parse("3.0.0").unwrap(),
                sha: "def".into(),
                ..Tag::default()
            },
        };

        let manifest = ManifestFile {
            path: Path::new("packages/a/Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".to_string(),
            content: content.into(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "package-a".to_string(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![package_b],
        };

        let result = cargo_toml.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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

        let package_b = ManifestPackage {
            name: "package-b".into(),
            release_type: ReleaseType::Rust,
            tag: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::parse("3.0.0").unwrap(),
                sha: "def".into(),
                ..Tag::default()
            },
        };

        let manifest = ManifestFile {
            path: Path::new("packages/a/Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".to_string(),
            content: content.into(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "package-a".to_string(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![package_b],
        };

        let result = cargo_toml.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("package-b = \"3.0.0\""));
    }

    #[test]
    fn skips_workspace_cargo_toml() {
        let cargo_toml = CargoToml::new();
        let content = r#"[workspace]
members = ["packages/*"]
"#;

        let manifest = ManifestFile {
            path: Path::new("packages/a/Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".to_string(),
            content: content.into(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "package-a".to_string(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = cargo_toml.update(&manifest).unwrap();

        assert!(result.is_none());
    }

    /// A virtual manifest owns `[workspace.dependencies]` and nothing
    /// else. Reaching that table must not index `[package]` into
    /// existence on the way past.
    #[test]
    fn syncs_workspace_dependencies_without_inventing_a_package_table() {
        let cargo_toml = CargoToml::new();
        let content = r#"[workspace]
members = ["packages/*"]

[workspace.dependencies]
package-b = "1.0.0"
"#;

        let package_b = ManifestPackage {
            name: "package-b".into(),
            release_type: ReleaseType::Rust,
            tag: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::new(3, 0, 0),
                sha: "def".into(),
                ..Tag::default()
            },
        };

        let manifest = ManifestFile {
            path: Path::new("Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".to_string(),
            content: content.into(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "workspace".to_string(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![package_b],
        };

        let result = cargo_toml.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("package-b = \"3.0.0\""));
        assert!(!updated.contains("[package]"));
    }

    /// A root package that also declares `[workspace]` used to be
    /// skipped wholesale, leaving the root crate permanently unbumped.
    #[test]
    fn bumps_a_root_package_that_also_declares_a_workspace() {
        let cargo_toml = CargoToml::new();
        let content = r#"[package]
name = "root"
version = "1.0.0"

[workspace]
members = ["crates/*"]
"#;

        let manifest = ManifestFile {
            path: Path::new("Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".to_string(),
            content: content.into(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "root".to_string(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = cargo_toml.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(updated.contains("members = [\"crates/*\"]"));
    }

    /// Probing for a dependency that isn't declared must not leave an
    /// empty table behind, at either level.
    #[test]
    fn does_not_create_a_dependencies_table_for_an_undeclared_peer() {
        let cargo_toml = CargoToml::new();
        let content = r#"[package]
name = "root"
version = "1.0.0"

[workspace]
members = ["crates/*"]
"#;

        let package_b = ManifestPackage {
            name: "package-b".into(),
            release_type: ReleaseType::Rust,
            tag: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::new(3, 0, 0),
                sha: "def".into(),
                ..Tag::default()
            },
        };

        let manifest = ManifestFile {
            path: Path::new("Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".to_string(),
            content: content.into(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "root".to_string(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![package_b],
        };

        let result = cargo_toml.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(!updated.contains("dependencies"));
    }

    /// `{ path = "../b" }` carries no version, so there is nothing to
    /// bump — and replacing the whole value with a bare version string
    /// would drop the path and break the build.
    #[test]
    fn leaves_a_path_only_dependency_intact() {
        let cargo_toml = CargoToml::new();
        let content = r#"[package]
name = "package-a"
version = "1.0.0"

[dependencies]
package-b = { path = "../b" }
"#;

        let package_b = ManifestPackage {
            name: "package-b".into(),
            release_type: ReleaseType::Rust,
            tag: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::new(3, 0, 0),
                sha: "def".into(),
                ..Tag::default()
            },
        };

        let manifest = ManifestFile {
            path: Path::new("packages/a/Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".to_string(),
            content: content.into(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "package-a".to_string(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![package_b],
        };

        let result = cargo_toml.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("package-b = { path = \"../b\" }"));
        assert!(!updated.contains("3.0.0"));
    }

    /// Nothing to write means no `FileChange`, so the forge is never
    /// asked to commit an identical file.
    #[test]
    fn returns_none_when_the_version_already_matches() {
        let cargo_toml = CargoToml::new();
        let content = r#"[package]
name = "my-package"
version = "2.0.0"
"#;

        let manifest = ManifestFile {
            path: Path::new("Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".into(),
            content: content.into(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "my-package".into(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        assert!(cargo_toml.update(&manifest).unwrap().is_none());
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
            path: Path::new("packages/a/Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".to_string(),
            content: content.into(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "package-a".to_string(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = cargo_toml.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(updated.contains("edition = \"2021\""));
        assert!(updated.contains("authors = [\"Test Author\"]"));
        assert!(updated.contains("serde = \"1.0\""));
    }

    #[test]
    fn process_package_returns_none_when_no_cargo_toml_files() {
        let cargo_toml = CargoToml::new();

        let manifest = ManifestFile {
            path: Path::new("Cargo.lock").to_path_buf(),
            basename: "Cargo.lock".to_string(),
            content: "version = 3\n".to_string(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "test".to_string(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = cargo_toml.update(&manifest).unwrap();

        assert!(result.is_none());
    }
}
