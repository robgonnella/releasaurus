use crate::{
    config::{package::GENERIC_VERSION_REGEX, release_type::ReleaseType},
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{generic::updater::GenericUpdater, traits::FileUpdater},
};

/// Handles version.go file parsing and version updates for Golang packages.
pub struct VersionGo {}

impl VersionGo {
    /// Create VersionGo handler for version.go version updates.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for VersionGo {
    fn default() -> Self {
        VersionGo::new()
    }
}

impl FileUpdater for VersionGo {
    /// Process version.go files for all Golang packages.
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename != "version.go"
            || !matches!(manifest.release_type, ReleaseType::Go)
        {
            return Ok(None);
        }

        Ok(GenericUpdater::update_manifest(
            manifest,
            &GENERIC_VERSION_REGEX,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use semver::Version;

    use crate::{
        config::release_type::ReleaseType,
        forge::request::Tag,
        packages::manifests::{ManifestFile, ManifestPackage},
    };

    use super::*;

    #[test]
    fn updates_const_version() {
        let version_go = VersionGo::new();

        let content = r#"
  const Version = "1.0.0"
"#;
        let manifest = ManifestFile {
            path: Path::new("version.go").to_path_buf(),
            basename: "version.go".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Go,
            owner: Some(ManifestPackage {
                name: "gopher".to_string(),
                release_type: ReleaseType::Go,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_go.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("const Version = \"2.0.0\""));
    }

    #[test]
    fn updates_var_version() {
        let version_go = VersionGo::new();

        let content = r#"
  var Version = "1.0.0"
"#;
        let manifest = ManifestFile {
            path: Path::new("version.go").to_path_buf(),
            basename: "version.go".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Go,
            owner: Some(ManifestPackage {
                name: "gopher".to_string(),
                release_type: ReleaseType::Go,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_go.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("var Version = \"2.0.0\""));
    }

    #[test]
    fn updates_all_caps_version() {
        let version_go = VersionGo::new();

        let content = r#"
  const VERSION = "1.0.0"
"#;
        let manifest = ManifestFile {
            path: Path::new("version.go").to_path_buf(),
            basename: "version.go".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Go,
            owner: Some(ManifestPackage {
                name: "gopher".to_string(),
                release_type: ReleaseType::Go,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_go.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("const VERSION = \"2.0.0\""));
    }

    #[test]
    fn updates_all_caps_version_2() {
        let version_go = VersionGo::new();

        let content = "package internal\n\nconst VERSION = \"0.0.1\"\n";
        let manifest = ManifestFile {
            path: Path::new("internal/version.go").to_path_buf(),
            basename: "version.go".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Go,
            owner: Some(ManifestPackage {
                name: "gopher".to_string(),
                release_type: ReleaseType::Go,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_go.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("const VERSION = \"2.0.0\""));
    }
}
