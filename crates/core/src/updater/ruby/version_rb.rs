use crate::{
    config::package::GENERIC_VERSION_REGEX,
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{generic::updater::GenericUpdater, traits::FileUpdater},
};

/// Handles version.rb file parsing and version updates for Ruby packages.
pub struct VersionRb {}

impl VersionRb {
    /// Create VersionRb handler for version.rb version updates.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for VersionRb {
    fn default() -> Self {
        VersionRb::new()
    }
}

impl FileUpdater for VersionRb {
    /// Process version.rb files for all Ruby packages.
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename != "version.rb" {
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

    use crate::{
        config::release_type::ReleaseType,
        forge::request::Tag,
        packages::manifests::{ManifestFile, ManifestPackage},
    };

    use super::*;

    #[test]
    fn updates_version_with_double_quotes() {
        let version_rb = VersionRb::new();
        let content = r#"module MyGem
  VERSION = "1.0.0"
end
"#;

        let manifest = ManifestFile {
            path: Path::new("lib/my_gem/version.rb").to_path_buf(),
            basename: "version.rb".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Ruby,
            owner: Some(ManifestPackage {
                name: "my-gem".to_string(),
                release_type: ReleaseType::Ruby,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_rb.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("VERSION = \"2.0.0\""));
    }

    #[test]
    fn updates_version_with_single_quotes() {
        let version_rb = VersionRb::new();
        let content = r#"module MyGem
  VERSION = '1.0.0'
end
"#;

        let manifest = ManifestFile {
            path: Path::new("lib/my_gem/version.rb").to_path_buf(),
            basename: "version.rb".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Ruby,
            owner: Some(ManifestPackage {
                name: "my-gem".to_string(),
                release_type: ReleaseType::Ruby,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_rb.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("VERSION = '2.0.0'"));
    }

    #[test]
    fn preserves_whitespace_formatting() {
        let version_rb = VersionRb::new();
        let content = r#"module MyGem
  VERSION   =   "1.0.0"
end
"#;

        let manifest = ManifestFile {
            path: Path::new("lib/my_gem/version.rb").to_path_buf(),
            basename: "version.rb".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Ruby,
            owner: Some(ManifestPackage {
                name: "my-gem".to_string(),
                release_type: ReleaseType::Ruby,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_rb.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("VERSION   =   \"2.0.0\""));
    }

    #[test]
    fn returns_none_when_no_version_constant() {
        let version_rb = VersionRb::new();
        let content = r#"module MyGem
  AUTHOR = "Test Author"
end
"#;

        let manifest = ManifestFile {
            path: Path::new("lib/my_gem/version.rb").to_path_buf(),
            basename: "version.rb".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Ruby,
            owner: Some(ManifestPackage {
                name: "my-gem".to_string(),
                release_type: ReleaseType::Ruby,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_rb.update(&manifest).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn preserves_other_content() {
        let version_rb = VersionRb::new();
        let content = r#"# frozen_string_literal: true

module MyGem
  # The current version
  VERSION = "1.0.0"

  # Other constants
  AUTHOR = "Test Author"
  HOMEPAGE = "https://example.com"
end
"#;

        let manifest = ManifestFile {
            path: Path::new("lib/my_gem/version.rb").to_path_buf(),
            basename: "version.rb".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Ruby,
            owner: Some(ManifestPackage {
                name: "my-gem".to_string(),
                release_type: ReleaseType::Ruby,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_rb.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("VERSION = \"2.0.0\""));
        assert!(updated.contains("# frozen_string_literal: true"));
        assert!(updated.contains("# The current version"));
        assert!(updated.contains("AUTHOR = \"Test Author\""));
        assert!(updated.contains("HOMEPAGE = \"https://example.com\""));
    }

    #[test]
    fn process_package_returns_none_when_no_version_rb_files() {
        let version_rb = VersionRb::new();

        let manifest = ManifestFile {
            path: Path::new("lib/my_gem.rb").to_path_buf(),
            basename: "my_gem.rb".to_string(),
            content: "module MyGem\n  # Main module\nend\n".to_string(),
            release_type: ReleaseType::Ruby,
            owner: Some(ManifestPackage {
                name: "my-gem".to_string(),
                release_type: ReleaseType::Ruby,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_rb.update(&manifest).unwrap();

        assert!(result.is_none());
    }
}
