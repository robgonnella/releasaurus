use regex::Regex;
use semver::Version;
use std::sync::LazyLock;

use crate::{
    Result,
    forge::request::{FileChange, FileUpdateType},
    updater::{
        manager::{ManifestFile, UpdaterPackage},
        traits::PackageUpdater,
    },
};

/// Default generic version matcher regex
static GENERIC_VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?mi)(?<start>.*version"?:?\s*=?\s*['"]?)(?<version>\d\.\d\.\d-?.*?)(?<end>['",].*)?$"#).unwrap()
});

/// Generic package updater for projects without specific language support.
pub struct GenericUpdater {}

impl GenericUpdater {
    /// Create generic updater that performs no version file updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Static fn to provide a generic regex version update for any manifest
    pub fn update_manifest(
        manifest: &ManifestFile,
        next_version: &Version,
    ) -> Option<FileChange> {
        if !GENERIC_VERSION_REGEX.is_match(&manifest.content) {
            return None;
        }

        let content = GENERIC_VERSION_REGEX
            .replace_all(
                &manifest.content,
                format!("${{start}}{}${{end}}", next_version,),
            )
            .to_string();

        if content != manifest.content {
            return Some(FileChange {
                path: manifest.path.clone(),
                content,
                update_type: FileUpdateType::Replace,
            });
        }

        None
    }
}

impl PackageUpdater for GenericUpdater {
    fn update(
        &self,
        _package: &UpdaterPackage,
        _workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::release_type::ReleaseType;

    use super::*;
    use semver::Version;

    fn create_manifest(content: &str) -> ManifestFile {
        ManifestFile {
            is_workspace: false,
            path: "test.txt".to_string(),
            basename: "test.txt".to_string(),
            content: content.to_string(),
        }
    }

    #[test]
    fn update_manifest_updates_version_with_double_quotes() {
        let manifest = create_manifest(r#"version = "1.0.0""#);
        let next_version = Version::parse("2.0.0").unwrap();

        let result = GenericUpdater::update_manifest(&manifest, &next_version);

        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.content, r#"version = "2.0.0""#);
        assert_eq!(change.path, "test.txt");
    }

    #[test]
    fn update_manifest_updates_version_with_single_quotes() {
        let manifest = create_manifest("version = '1.0.0'");
        let next_version = Version::parse("2.0.0").unwrap();

        let result = GenericUpdater::update_manifest(&manifest, &next_version);

        assert!(result.is_some());
        assert!(result.unwrap().content.contains("'2.0.0'"));
    }

    #[test]
    fn update_manifest_updates_version_with_colon() {
        let manifest = create_manifest(r#""version": "1.0.0""#);
        let next_version = Version::parse("2.0.0").unwrap();

        let result = GenericUpdater::update_manifest(&manifest, &next_version);

        assert!(result.is_some());
        assert_eq!(result.unwrap().content, r#""version": "2.0.0""#);
    }

    #[test]
    fn update_manifest_preserves_whitespace() {
        let manifest = create_manifest("version   =   \"1.0.0\"");
        let next_version = Version::parse("2.0.0").unwrap();

        let result = GenericUpdater::update_manifest(&manifest, &next_version);

        assert!(result.is_some());
        assert_eq!(result.unwrap().content, "version   =   \"2.0.0\"");
    }

    #[test]
    fn update_manifest_updates_version_with_prerelease() {
        let manifest = create_manifest(r#"version = "1.0.0-alpha.1""#);
        let next_version = Version::parse("2.0.0-beta.2").unwrap();

        let result = GenericUpdater::update_manifest(&manifest, &next_version);

        assert!(result.is_some());
        assert!(result.unwrap().content.contains("2.0.0-beta.2"));
    }

    #[test]
    fn update_manifest_handles_multiline_content() {
        let manifest = create_manifest(
            "name = \"my-package\"\nversion = \"1.0.0\"\nauthor = \"Test\"",
        );
        let next_version = Version::parse("2.0.0").unwrap();

        let result = GenericUpdater::update_manifest(&manifest, &next_version);

        assert!(result.is_some());
        let content = result.unwrap().content;
        assert!(content.contains("version = \"2.0.0\""));
        assert!(content.contains("name = \"my-package\""));
        assert!(content.contains("author = \"Test\""));
    }

    #[test]
    fn update_manifest_returns_none_when_no_version_pattern() {
        let manifest = create_manifest("name = \"my-package\"");
        let next_version = Version::parse("2.0.0").unwrap();

        let result = GenericUpdater::update_manifest(&manifest, &next_version);

        assert!(result.is_none());
    }

    #[test]
    fn update_manifest_returns_none_when_version_unchanged() {
        let manifest = create_manifest(r#"version = "2.0.0""#);
        let next_version = Version::parse("2.0.0").unwrap();

        let result = GenericUpdater::update_manifest(&manifest, &next_version);

        assert!(result.is_none());
    }

    #[test]
    fn update_manifest_is_case_insensitive() {
        let manifest = create_manifest(r#"VERSION = "1.0.0""#);
        let next_version = Version::parse("2.0.0").unwrap();

        let result = GenericUpdater::update_manifest(&manifest, &next_version);

        assert!(result.is_some());
        assert!(result.unwrap().content.contains("2.0.0"));
    }

    #[test]
    fn update_manifest_updates_yaml_format() {
        let manifest = create_manifest(
            "metadata:\n  version: \"1.0.0\"\n  description: \"My app\"",
        );
        let next_version = Version::parse("2.5.3").unwrap();

        let result = GenericUpdater::update_manifest(&manifest, &next_version);

        assert!(result.is_some());
        let content = result.unwrap().content;
        assert!(content.contains("version: \"2.5.3\""));
        assert!(content.contains("metadata:"));
        assert!(content.contains("description: \"My app\""));
    }

    #[test]
    fn update_manifest_updates_go_version_file() {
        let manifest = create_manifest(
            "package main\n\nconst Version = \"1.0.0\"\nconst AppName = \"myapp\"",
        );
        let next_version = Version::parse("3.2.1").unwrap();

        let result = GenericUpdater::update_manifest(&manifest, &next_version);

        assert!(result.is_some());
        let content = result.unwrap().content;
        assert!(content.contains("const Version = \"3.2.1\""));
        assert!(content.contains("package main"));
        assert!(content.contains("const AppName = \"myapp\""));
    }

    #[test]
    fn package_updater_update_returns_none() {
        let updater = GenericUpdater::new();
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![],
            next_version: crate::analyzer::release::Tag {
                sha: "abc".to_string(),
                name: "v1.0.0".to_string(),
                semver: Version::parse("1.0.0").unwrap(),
                timestamp: None,
            },
            release_type: ReleaseType::Generic,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_none());
    }
}
