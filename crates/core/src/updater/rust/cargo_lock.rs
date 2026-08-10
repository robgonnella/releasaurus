use toml_edit::{DocumentMut, value};

use crate::{
    forge::request::{FileChange, FileUpdateType},
    packages::manifests::ManifestFile,
    result::Result,
    updater::traits::FileUpdater,
};

/// Handles Cargo.lock file parsing and version synchronization for Rust
/// workspace dependencies.
pub struct CargoLock {}

impl CargoLock {
    /// Create Cargo.lock handler for lockfile version updates.
    pub fn new() -> Self {
        Self {}
    }

    fn load_doc(&self, content: &str) -> Result<DocumentMut> {
        let doc = content.parse::<DocumentMut>()?;
        Ok(doc)
    }
}

impl Default for CargoLock {
    fn default() -> Self {
        CargoLock::new()
    }
}

impl FileUpdater for CargoLock {
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename != "Cargo.lock" {
            return Ok(None);
        }

        let mut lock_doc = self.load_doc(&manifest.content)?;

        let Some(doc_packages) = lock_doc["package"].as_array_of_tables_mut()
        else {
            return Ok(None);
        };

        // `releasing` includes the owner, so a shared workspace lock is
        // rewritten once carrying every member's bump. There is no
        // driving package to special-case.
        let mut changed = false;

        for pkg in manifest.releasing.iter() {
            let Some(found) = doc_packages.iter_mut().find(|p| {
                let doc_package_name =
                    p.get("name").and_then(|item| item.as_str()).unwrap_or("");
                doc_package_name == pkg.name
            }) else {
                continue;
            };

            let next_version = pkg.tag.semver.to_string();

            if found.get("version").and_then(|v| v.as_str())
                != Some(next_version.as_str())
            {
                found["version"] = value(next_version);
                changed = true;
            }
        }

        if !changed {
            return Ok(None);
        }

        Ok(Some(FileChange {
            path: manifest.path.to_string_lossy().to_string(),
            content: lock_doc.to_string(),
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

    fn package(name: &str, version: &str) -> ManifestPackage {
        ManifestPackage {
            name: name.to_string(),
            release_type: ReleaseType::Rust,
            tag: Tag {
                name: format!("v{version}"),
                semver: semver::Version::parse(version).unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
        }
    }

    /// `releasing` is owner-inclusive in production, so fixtures build it
    /// that way rather than naming the owner twice.
    fn manifest(
        path: &str,
        content: &str,
        owner: ManifestPackage,
        peers: Vec<ManifestPackage>,
    ) -> ManifestFile {
        let mut releasing = vec![owner.clone()];
        releasing.extend(peers);

        let path = Path::new(path);

        ManifestFile {
            basename: path.file_name().unwrap().to_string_lossy().to_string(),
            path: path.to_path_buf(),
            content: content.to_string(),
            release_type: ReleaseType::Rust,
            owner: Some(owner),
            releasing,
        }
    }

    #[test]
    fn updates_workspace_package_version() {
        let cargo_lock = CargoLock::new();
        let content = r#"version = 3

[[package]]
name = "my-package"
version = "1.0.0"
"#;

        let manifest = manifest(
            "Cargo.lock",
            content,
            package("my-package", "2.0.0"),
            vec![],
        );

        let result = cargo_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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

        let manifest = manifest(
            "Cargo.lock",
            content,
            package("package-a", "2.0.0"),
            vec![package("package-b", "3.0.0")],
        );

        let result = cargo_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(updated.contains("version = \"3.0.0\""));
    }

    /// A shared workspace lock has no owner when the workspace root is
    /// not itself being released. Every member still gets bumped.
    #[test]
    fn bumps_every_member_of_an_unowned_workspace_lock() {
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
            path: Path::new("Cargo.lock").to_path_buf(),
            basename: "Cargo.lock".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Rust,
            owner: None,
            releasing: vec![
                package("package-a", "2.0.0"),
                package("package-b", "3.0.0"),
            ],
        };

        let result = cargo_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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

        let manifest = manifest(
            "Cargo.lock",
            content,
            package("my-package", "2.0.0"),
            vec![],
        );

        let result = cargo_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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

        let manifest = manifest(
            "Cargo.lock",
            content,
            package("my-package", "2.0.0"),
            vec![],
        );

        let result = cargo_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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

        let manifest = manifest(
            "Cargo.lock",
            "version = 3\n",
            package("my-package", "2.0.0"),
            vec![],
        );

        assert!(cargo_lock.update(&manifest).unwrap().is_none());
    }

    /// Nothing to write means no `FileChange`, so the forge is never asked
    /// to commit an identical lock.
    #[test]
    fn returns_none_when_every_version_already_matches() {
        let cargo_lock = CargoLock::new();
        let content = r#"version = 3

[[package]]
name = "my-package"
version = "2.0.0"
"#;

        let manifest = manifest(
            "Cargo.lock",
            content,
            package("my-package", "2.0.0"),
            vec![],
        );

        assert!(cargo_lock.update(&manifest).unwrap().is_none());
    }

    #[test]
    fn process_package_returns_none_when_no_cargo_lock_files() {
        let cargo_lock = CargoLock::new();

        let manifest = manifest(
            "Cargo.toml",
            "[package]\nname = \"my-package\"\nversion = \"1.0.0\"\n",
            package("my-package", "2.0.0"),
            vec![],
        );

        assert!(cargo_lock.update(&manifest).unwrap().is_none());
    }
}
