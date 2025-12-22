//! Cargo updater for handling rust projects
use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        manager::UpdaterPackage,
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

impl PackageUpdater for RustUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        if let Some(changes) = self
            .cargo_toml
            .process_package(package, &workspace_packages)?
        {
            file_changes.extend(changes);
        }

        if let Some(changes) = self
            .cargo_lock
            .process_package(package, &workspace_packages)?
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
        analyzer::release::Tag,
        config::release_type::ReleaseType,
        updater::manager::{ManifestFile, UpdaterPackage},
    };

    #[test]
    fn processes_rust_project() {
        let updater = RustUpdater::new();
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
            manifest_files: vec![manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Rust,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_some());
        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_rust_files() {
        let updater = RustUpdater::new();
        let manifest = ManifestFile {
            is_workspace: false,
            path: "package.json".to_string(),
            basename: "package.json".to_string(),
            content: r#"{"version":"1.0.0"}"#.to_string(),
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

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_none());
    }
}
