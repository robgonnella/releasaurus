//! Cargo updater for handling rust projects

use crate::{
    config::release_type::ReleaseType,
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{
        composite::CompositeUpdater,
        rust::{cargo_lock::CargoLock, cargo_toml::CargoToml},
        traits::FileUpdater,
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

impl Default for RustUpdater {
    fn default() -> Self {
        RustUpdater::new()
    }
}

impl FileUpdater for RustUpdater {
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if !matches!(manifest.release_type, ReleaseType::Rust) {
            return Ok(None);
        }
        self.composite.update(manifest)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        config::release_type::ReleaseType,
        forge::request::Tag,
        packages::manifests::{ManifestFile, ManifestPackage},
        result::ReleasaurusError,
    };

    use super::*;

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
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = updater.update(&manifest).unwrap();

        assert!(result.unwrap().content.contains("2.0.0"));
    }

    #[test]
    fn a_cargo_toml_that_will_not_parse_is_an_error() {
        let updater = RustUpdater::new();

        let manifest = ManifestFile {
            path: Path::new("Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".to_string(),
            content: r#"{"version":"1.0.0"}"#.to_string(),
            release_type: ReleaseType::Rust,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Rust,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let err = updater.update(&manifest).unwrap_err();

        assert!(matches!(err, ReleasaurusError::TomlEditError(_)));
    }

    /// Dispatch keys off the file's own release type, so a Node manifest
    /// that somehow reaches the Rust updater is left alone rather than
    /// parsed as TOML.
    #[test]
    fn ignores_a_manifest_of_another_release_type() {
        let updater = RustUpdater::new();

        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".to_string(),
            content: r#"{"version":"1.0.0"}"#.to_string(),
            release_type: ReleaseType::Node,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Node,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        assert!(updater.update(&manifest).unwrap().is_none());
    }
}
