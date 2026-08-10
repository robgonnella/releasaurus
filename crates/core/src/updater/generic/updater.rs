use regex::Regex;

use crate::{
    forge::request::{FileChange, FileUpdateType},
    packages::manifests::ManifestFile,
    result::Result,
    updater::traits::FileUpdater,
};

/// Generic package updater for projects without specific language support.
#[derive(Default)]
pub struct GenericUpdater {}

impl GenericUpdater {
    /// Static fn to provide a generic regex version update for any manifest
    ///
    /// Every caller writes a single package's own version, so a manifest
    /// no releasing package owns has nothing to substitute.
    pub fn update_manifest(
        manifest: &ManifestFile,
        version_regex: &Regex,
    ) -> Option<FileChange> {
        let owner = manifest.owner.as_ref()?;

        if !version_regex.is_match(&manifest.content) {
            return None;
        }

        let content = version_regex
            .replace_all(&manifest.content, |caps: &regex::Captures| {
                // Replace only the version capture group, preserving
                // surrounding context
                let full_match = &caps[0];
                let version_match = &caps["version"];
                full_match.replacen(
                    version_match,
                    &owner.tag.semver.to_string(),
                    1,
                )
            })
            .to_string();

        if content != manifest.content {
            return Some(FileChange {
                path: manifest.path.to_string_lossy().to_string(),
                content,
                update_type: FileUpdateType::Replace,
            });
        }

        None
    }
}

impl FileUpdater for GenericUpdater {
    fn update(&self, _manifest: &ManifestFile) -> Result<Option<FileChange>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use semver::Version;
    use std::path::Path;

    use crate::{
        config::{package::GENERIC_VERSION_REGEX, release_type::ReleaseType},
        forge::request::Tag,
        packages::manifests::ManifestPackage,
    };

    use super::*;

    fn create_manifest(content: &str) -> ManifestFile {
        ManifestFile {
            path: Path::new("test.txt").to_path_buf(),
            basename: "test.txt".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Generic,
            owner: Some(ManifestPackage {
                name: "test-pkg".into(),
                release_type: ReleaseType::Generic,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: Version::new(2, 0, 0),
                    sha: "abc123".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        }
    }

    #[test]
    fn update_manifest_updates_version_with_double_quotes() {
        let manifest = create_manifest(r#"version = "1.0.0""#);

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX);

        let change = result.unwrap();
        assert_eq!(change.content, r#"version = "2.0.0""#);
        assert_eq!(change.path, "test.txt");
    }

    #[test]
    fn update_manifest_updates_version_with_single_quotes() {
        let manifest = create_manifest("version = '1.0.0'");

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX);

        assert!(result.unwrap().content.contains("'2.0.0'"));
    }

    #[test]
    fn update_manifest_updates_version_with_colon() {
        let manifest = create_manifest(r#""version": "1.0.0""#);

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX);

        assert_eq!(result.unwrap().content, r#""version": "2.0.0""#);
    }

    #[test]
    fn update_manifest_preserves_whitespace() {
        let manifest = create_manifest("version   =   \"1.0.0\"");

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX);

        assert_eq!(result.unwrap().content, "version   =   \"2.0.0\"");
    }

    #[test]
    fn update_manifest_updates_version_with_prerelease() {
        let mut manifest = create_manifest(r#"version = "1.0.0-alpha.1""#);
        manifest.owner.as_mut().unwrap().tag.name = "v2.0.0-beta.2".into();
        manifest.owner.as_mut().unwrap().tag.semver =
            Version::parse("2.0.0-beta.2").unwrap();

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX);

        assert!(result.unwrap().content.contains("2.0.0-beta.2"));
    }

    #[test]
    fn update_manifest_handles_multiline_content() {
        let manifest = create_manifest(
            "name = \"my-package\"\nversion = \"1.0.0\"\nauthor = \"Test\"",
        );

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX);

        let content = result.unwrap().content;
        assert!(content.contains("version = \"2.0.0\""));
        assert!(content.contains("name = \"my-package\""));
        assert!(content.contains("author = \"Test\""));
    }

    #[test]
    fn update_manifest_returns_none_when_no_version_pattern() {
        let manifest = create_manifest("name = \"my-package\"");

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX);

        assert!(result.is_none());
    }

    #[test]
    fn update_manifest_returns_none_when_version_unchanged() {
        let manifest = create_manifest(r#"version = "2.0.0""#);

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX);

        assert!(result.is_none());
    }

    #[test]
    fn update_manifest_is_case_insensitive() {
        let manifest = create_manifest(r#"VERSION = "1.0.0""#);

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX);

        assert!(result.unwrap().content.contains("2.0.0"));
    }

    #[test]
    fn update_manifest_updates_yaml_format() {
        let mut manifest = create_manifest(
            "metadata:\n  version: \"1.0.0\"\n  description: \"My app\"",
        );
        manifest.owner.as_mut().unwrap().tag.name = "v2.5.3".into();
        manifest.owner.as_mut().unwrap().tag.semver = Version::new(2, 5, 3);

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX);

        let content = result.unwrap().content;
        assert!(content.contains("version: \"2.5.3\""));
        assert!(content.contains("metadata:"));
        assert!(content.contains("description: \"My app\""));
    }

    #[test]
    fn update_manifest_updates_go_version_file() {
        let mut manifest = create_manifest(
            "package main\n\nconst Version = \"1.0.0\"\nconst AppName = \"myapp\"",
        );
        manifest.owner.as_mut().unwrap().tag.name = "v3.2.1".into();
        manifest.owner.as_mut().unwrap().tag.semver = Version::new(3, 2, 1);

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX);

        let content = result.unwrap().content;
        assert!(content.contains("const Version = \"3.2.1\""));
        assert!(content.contains("package main"));
        assert!(content.contains("const AppName = \"myapp\""));
    }

    #[test]
    fn package_updater_update_returns_none() {
        let updater = GenericUpdater::default();
        let manifest = ManifestFile::default();
        let result = updater.update(&manifest).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn update_manifest_handles_multi_digit_versions() {
        let mut manifest = create_manifest(r#"version = "10.200.3""#);
        manifest.owner.as_mut().unwrap().tag.name = "v11.0.0".into();
        manifest.owner.as_mut().unwrap().tag.semver = Version::new(11, 0, 0);

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX)
                .unwrap();

        assert_eq!(result.content, r#"version = "11.0.0""#);
    }
}
