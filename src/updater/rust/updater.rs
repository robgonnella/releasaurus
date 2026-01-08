//! Cargo updater for handling rust projects
use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        composite::CompositeUpdater,
        manager::UpdaterPackage,
        rust::{cargo_lock::CargoLock, cargo_toml::CargoToml},
        traits::PackageUpdater,
    },
};

/// Updates Cargo.toml and Cargo.lock files for Rust packages, handling
/// workspace dependencies and version synchronization.
pub struct RustUpdater {
    composite: CompositeUpdater,
}

impl RustUpdater {
    /// Create Rust updater with Cargo.toml and Cargo.lock handlers.
    pub fn new() -> Self {
        Self {
            composite: CompositeUpdater::new(vec![
                Box::new(CargoToml::new()),
                Box::new(CargoLock::new()),
            ]),
        }
    }
}

impl PackageUpdater for RustUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        self.composite.update(package, workspace_packages)
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, rc::Rc};

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
    fn processes_rust_project() {
        let updater = RustUpdater::new();
        let content = r#"[package]
name = "my-package"
version = "1.0.0"
"#;
        let manifest = ManifestFile {
            path: Path::new("Cargo.toml").to_path_buf(),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
        };

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_rust_files() {
        let updater = RustUpdater::new();
        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
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
            updater: Rc::new(Updater::new(ReleaseType::Rust)),
        };

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }
}
