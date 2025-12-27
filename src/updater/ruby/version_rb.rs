use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        generic::updater::GenericUpdater, manager::UpdaterPackage,
        traits::PackageUpdater,
    },
};

/// Handles version.rb file parsing and version updates for Ruby packages.
pub struct VersionRb {}

impl VersionRb {
    /// Create VersionRb handler for version.rb version updates.
    pub fn new() -> Self {
        Self {}
    }
}

impl PackageUpdater for VersionRb {
    /// Process version.rb files for all Ruby packages.
    fn update(
        &self,
        package: &UpdaterPackage,
        _workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.basename != "version.rb" {
                continue;
            }

            if let Some(change) = GenericUpdater::update_manifest(
                manifest,
                &package.next_version.semver,
            ) {
                file_changes.push(change);
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

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
    fn updates_version_with_double_quotes() {
        let version_rb = VersionRb::new();
        let content = r#"module MyGem
  VERSION = "1.0.0"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "lib/my_gem/version.rb".to_string(),
            basename: "version.rb".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Ruby)),
        };

        let result = version_rb.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
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
            is_workspace: false,
            path: "lib/my_gem/version.rb".to_string(),
            basename: "version.rb".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Ruby)),
        };

        let result = version_rb.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
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
            is_workspace: false,
            path: "lib/my_gem/version.rb".to_string(),
            basename: "version.rb".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Ruby)),
        };

        let result = version_rb.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
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
            is_workspace: false,
            path: "lib/my_gem/version.rb".to_string(),
            basename: "version.rb".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Ruby)),
        };

        let result = version_rb.update(&package, &[]).unwrap();

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
            is_workspace: false,
            path: "lib/my_gem/version.rb".to_string(),
            basename: "version.rb".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Ruby)),
        };

        let result = version_rb.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("VERSION = \"2.0.0\""));
        assert!(updated.contains("# frozen_string_literal: true"));
        assert!(updated.contains("# The current version"));
        assert!(updated.contains("AUTHOR = \"Test Author\""));
        assert!(updated.contains("HOMEPAGE = \"https://example.com\""));
    }

    #[test]
    fn process_package_handles_multiple_version_rb_files() {
        let version_rb = VersionRb::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            path: "gems/a/lib/gem_a/version.rb".to_string(),
            basename: "version.rb".to_string(),
            content: "module GemA\n  VERSION = \"1.0.0\"\nend\n".to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            path: "gems/b/lib/gem_b/version.rb".to_string(),
            basename: "version.rb".to_string(),
            content: "module GemB\n  VERSION = \"1.0.0\"\nend\n".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Ruby)),
        };

        let result = version_rb.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[test]
    fn process_package_returns_none_when_no_version_rb_files() {
        let version_rb = VersionRb::new();
        let manifest = ManifestFile {
            is_workspace: false,
            path: "lib/my_gem.rb".to_string(),
            basename: "my_gem.rb".to_string(),
            content: "module MyGem\n  # Main module\nend\n".to_string(),
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
            updater: Rc::new(Updater::new(ReleaseType::Ruby)),
        };

        let result = version_rb.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }
}
