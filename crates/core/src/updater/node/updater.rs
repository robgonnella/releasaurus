use crate::{
    config::release_type::ReleaseType,
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{
        composite::CompositeUpdater,
        node::{
            package_json::PackageJson, package_lock::PackageLock,
            yarn_lock::YarnLock,
        },
        traits::FileUpdater,
    },
};

/// Node.js package updater for npm, yarn, and pnpm projects.
pub struct NodeUpdater {
    composite: CompositeUpdater,
}

impl NodeUpdater {
    /// Create Node.js updater for package.json and lock file management.
    pub fn new() -> Self {
        Self {
            composite: CompositeUpdater::new(vec![
                Box::new(PackageJson::new()),
                Box::new(PackageLock::new()),
                Box::new(YarnLock::new()),
            ]),
        }
    }
}

impl Default for NodeUpdater {
    fn default() -> Self {
        NodeUpdater::new()
    }
}

impl FileUpdater for NodeUpdater {
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if !matches!(manifest.release_type, ReleaseType::Node) {
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
    fn processes_node_project() {
        let updater = NodeUpdater::new();
        let content = r#"{"name":"my-package","version":"1.0.0"}"#;

        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".to_string(),
            content: content.to_string(),
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

        let result = updater.update(&manifest).unwrap();

        assert!(result.unwrap().content.contains("2.0.0"));
    }

    #[test]
    fn a_package_json_that_will_not_parse_is_an_error() {
        let updater = NodeUpdater::new();

        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".to_string(),
            content: "[package]\nversion = \"1.0.0\"\n".to_string(),
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

        let err = updater.update(&manifest).unwrap_err();

        assert!(matches!(err, ReleasaurusError::JsonParseError(_)));
    }
}
