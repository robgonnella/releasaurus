use regex::{Captures, Regex};

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

        let content = Self::replace_versions(
            &manifest.content,
            version_regex,
            &owner.tag.version.to_string(),
        );

        if content != manifest.content {
            return Some(FileChange {
                path: manifest.path.to_string_lossy().to_string(),
                content,
                update_type: FileUpdateType::Replace,
            });
        }

        None
    }

    /// Replaces the `version` capture group of every match of
    /// `version_regex` in `content` with `new_version`.
    pub fn replace_versions(
        content: &str,
        version_regex: &Regex,
        new_version: &str,
    ) -> String {
        version_regex
            .replace_all(content, |caps: &Captures| {
                Self::replace_capture(caps, new_version)
            })
            .to_string()
    }

    /// One match of a version regex with its `version` capture group
    /// replaced by `new_version`. Replaces by position: the same text can
    /// also appear earlier in the match (a key containing the value).
    pub fn replace_capture(caps: &Captures, new_version: &str) -> String {
        let full = caps.get(0).unwrap();
        let version = caps.name("version").unwrap();
        let start = version.start() - full.start();
        let end = version.end() - full.start();

        format!(
            "{}{new_version}{}",
            &full.as_str()[..start],
            &full.as_str()[end..]
        )
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
        forge::request::{Tag, TagVersion},
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
                    version: TagVersion::Semantic(Version::new(2, 0, 0)),
                    sha: "abc123".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        }
    }

    /// `on` also appears inside the key, so only replacing the captured
    /// value by position gets it right.
    #[test]
    fn update_manifest_replaces_the_captured_value_not_an_earlier_copy() {
        let manifest = create_manifest("version: on");

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX);

        assert_eq!(result.unwrap().content, "version: 2.0.0");
    }

    #[test]
    fn default_regex_replaces_a_value_of_any_shape() {
        for (before, after) in [
            ("version = 26.09", "version = 2.0.0"),
            (r#"version = "20260901""#, r#"version = "2.0.0""#),
            ("version: latest", "version: 2.0.0"),
        ] {
            let result = GenericUpdater::update_manifest(
                &create_manifest(before),
                &GENERIC_VERSION_REGEX,
            );

            assert_eq!(result.unwrap().content, after);
        }
    }

    /// The key must start the line (after indentation) and be followed by
    /// `:` or `=`.
    #[test]
    fn default_regex_leaves_other_lines_mentioning_version_alone() {
        for content in [
            "apiVersion: 1.0.0",
            "const Version = \"1.0.0\"",
            "spec.required_ruby_version = \">= 2.7\"",
            "install version 1.0.0",
            "# version = 1.0.0",
        ] {
            let result = GenericUpdater::update_manifest(
                &create_manifest(content),
                &GENERIC_VERSION_REGEX,
            );

            assert!(result.is_none(), "{content}");
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
        manifest.owner.as_mut().unwrap().tag.version =
            TagVersion::Semantic(Version::parse("2.0.0-beta.2").unwrap());

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
        for (before, after) in [
            (r#"VERSION = "1.0.0""#, r#"VERSION = "2.0.0""#),
            ("Version: 1.0.0", "Version: 2.0.0"),
        ] {
            let result = GenericUpdater::update_manifest(
                &create_manifest(before),
                &GENERIC_VERSION_REGEX,
            );

            assert_eq!(result.unwrap().content, after);
        }
    }

    #[test]
    fn update_manifest_updates_yaml_format() {
        let mut manifest = create_manifest(
            "metadata:\n  version: \"1.0.0\"\n  description: \"My app\"",
        );
        manifest.owner.as_mut().unwrap().tag.name = "v2.5.3".into();
        manifest.owner.as_mut().unwrap().tag.version =
            TagVersion::Semantic(Version::new(2, 5, 3));

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX);

        let content = result.unwrap().content;
        assert!(content.contains("version: \"2.5.3\""));
        assert!(content.contains("metadata:"));
        assert!(content.contains("description: \"My app\""));
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
        manifest.owner.as_mut().unwrap().tag.version =
            TagVersion::Semantic(Version::new(11, 0, 0));

        let result =
            GenericUpdater::update_manifest(&manifest, &GENERIC_VERSION_REGEX)
                .unwrap();

        assert_eq!(result.content, r#"version = "11.0.0""#);
    }
}
