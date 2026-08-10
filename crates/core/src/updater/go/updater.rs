use crate::{
    config::release_type::ReleaseType,
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{go::version_go::VersionGo, traits::FileUpdater},
};

/// Golang package updater
pub struct GoUpdater {
    version_go: VersionGo,
}

impl GoUpdater {
    /// Create Golang updater.
    pub fn new() -> Self {
        Self {
            version_go: VersionGo::new(),
        }
    }
}

impl Default for GoUpdater {
    fn default() -> Self {
        GoUpdater::new()
    }
}

impl FileUpdater for GoUpdater {
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if !matches!(manifest.release_type, ReleaseType::Go) {
            return Ok(None);
        }
        self.version_go.update(manifest)
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
    fn processes_go_project() {
        let updater = GoUpdater::new();
        let content = r#"
const Version = "1.0.0"
"#;
        let manifest = ManifestFile {
            path: Path::new("version.go").to_path_buf(),
            basename: "version.go".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Go,
            owner: Some(ManifestPackage {
                name: "gopher".into(),
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

        let result = updater.update(&manifest).unwrap();

        assert!(result.unwrap().content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_go_files() {
        let updater = GoUpdater::new();

        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".to_string(),
            content: r#"{"version":"1.0.0"}"#.to_string(),
            release_type: ReleaseType::Go,
            owner: Some(ManifestPackage {
                name: "gopher".into(),
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

        let result = updater.update(&manifest).unwrap();

        assert!(result.is_none());
    }
}
