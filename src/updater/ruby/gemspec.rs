use log::*;
use std::path::Path;

use crate::{
    Result,
    forge::request::FileChange,
    updater::{generic::updater::GenericUpdater, manager::UpdaterPackage},
};

/// Handles .gemspec file parsing and version updates for Ruby packages.
pub struct Gemspec {}

impl Gemspec {
    /// Create Gemspec handler for .gemspec version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Process gemspec files for all Ruby packages.
    pub fn process_packages(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        for manifest in package.manifest_files.iter() {
            let file_path = Path::new(&manifest.basename);

            if let Some(file_ext) = file_path.extension() {
                if file_ext.display().to_string() != "gemspec" {
                    continue;
                }

                info!("processing gemspec file: {}", manifest.basename);

                if let Some(change) = GenericUpdater::update_manifest(
                    manifest,
                    &package.next_version.semver,
                ) {
                    file_changes.push(change);
                }
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
    use super::*;
    use crate::{
        analyzer::release::Tag,
        config::release_type::ReleaseType,
        updater::manager::{ManifestFile, UpdaterPackage},
    };

    #[test]
    fn updates_version_with_spec_prefix_and_double_quotes() {
        let gemspec = Gemspec::new();
        let content = r#"Gem::Specification.new do |spec|
  spec.name = "my-gem"
  spec.version = "1.0.0"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "my-gem.gemspec".to_string(),
            basename: "my-gem.gemspec".to_string(),
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
            release_type: ReleaseType::Ruby,
        };

        let result = gemspec.process_packages(&package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("spec.version = \"2.0.0\""));
    }

    #[test]
    fn updates_version_with_s_prefix() {
        let gemspec = Gemspec::new();
        let content = r#"Gem::Specification.new do |s|
  s.name = "my-gem"
  s.version = "1.0.0"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "my-gem.gemspec".to_string(),
            basename: "my-gem.gemspec".to_string(),
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
            release_type: ReleaseType::Ruby,
        };

        let result = gemspec.process_packages(&package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("s.version = \"2.0.0\""));
    }

    #[test]
    fn updates_version_with_single_quotes() {
        let gemspec = Gemspec::new();
        let content = r#"Gem::Specification.new do |spec|
  spec.name = 'my-gem'
  spec.version = '1.0.0'
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "my-gem.gemspec".to_string(),
            basename: "my-gem.gemspec".to_string(),
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
            release_type: ReleaseType::Ruby,
        };

        let result = gemspec.process_packages(&package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("spec.version = '2.0.0'"));
    }

    #[test]
    fn preserves_whitespace_formatting() {
        let gemspec = Gemspec::new();
        let content = r#"Gem::Specification.new do |spec|
  spec.version   =   "1.0.0"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "my-gem.gemspec".to_string(),
            basename: "my-gem.gemspec".to_string(),
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
            release_type: ReleaseType::Ruby,
        };

        let result = gemspec.process_packages(&package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("spec.version   =   \"2.0.0\""));
    }

    #[test]
    fn preserves_other_fields() {
        let gemspec = Gemspec::new();
        let content = r#"Gem::Specification.new do |spec|
  spec.name = "my-gem"
  spec.version = "1.0.0"
  spec.authors = ["Test Author"]
  spec.summary = "A test gem"
  spec.files = Dir["lib/**/*"]

  spec.add_dependency "rails", "~> 7.0"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "my-gem.gemspec".to_string(),
            basename: "my-gem.gemspec".to_string(),
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
            release_type: ReleaseType::Ruby,
        };

        let result = gemspec.process_packages(&package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("spec.version = \"2.0.0\""));
        assert!(updated.contains("spec.name = \"my-gem\""));
        assert!(updated.contains("spec.authors = [\"Test Author\"]"));
        assert!(updated.contains("spec.summary = \"A test gem\""));
        assert!(updated.contains("spec.add_dependency \"rails\", \"~> 7.0\""));
    }

    #[test]
    fn process_packages_handles_multiple_gemspec_files() {
        let gemspec = Gemspec::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            path: "gems/a/gem-a.gemspec".to_string(),
            basename: "gem-a.gemspec".to_string(),
            content: "Gem::Specification.new do |spec|\n  spec.version = \"1.0.0\"\nend\n".to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            path: "gems/b/gem-b.gemspec".to_string(),
            basename: "gem-b.gemspec".to_string(),
            content: "Gem::Specification.new do |spec|\n  spec.version = \"1.0.0\"\nend\n".to_string(),
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
            release_type: ReleaseType::Ruby,
        };

        let result = gemspec.process_packages(&package).unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[test]
    fn process_packages_returns_none_when_no_gemspec_files() {
        let gemspec = Gemspec::new();
        let manifest = ManifestFile {
            is_workspace: false,
            path: "Gemfile".to_string(),
            basename: "Gemfile".to_string(),
            content: "source 'https://rubygems.org'\ngem 'rails'".to_string(),
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
            release_type: ReleaseType::Ruby,
        };

        let result = gemspec.process_packages(&package).unwrap();

        assert!(result.is_none());
    }
}
