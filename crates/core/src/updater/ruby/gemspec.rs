use std::path::Path;

use crate::{
    config::package::GENERIC_VERSION_REGEX,
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{generic::updater::GenericUpdater, traits::FileUpdater},
};

/// Handles .gemspec file parsing and version updates for Ruby packages.
pub struct Gemspec {}

impl Gemspec {
    /// Create Gemspec handler for .gemspec version updates.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Gemspec {
    fn default() -> Self {
        Gemspec::new()
    }
}

impl FileUpdater for Gemspec {
    /// Process gemspec files for all Ruby packages.
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        let file_path = Path::new(&manifest.basename);

        if let Some(file_ext) = file_path.extension() {
            if file_ext.to_string_lossy() != "gemspec" {
                return Ok(None);
            }

            log::info!("processing gemspec file: {}", manifest.basename);

            return Ok(GenericUpdater::update_manifest(
                manifest,
                &GENERIC_VERSION_REGEX,
            ));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::release_type::ReleaseType,
        forge::request::Tag,
        packages::manifests::{ManifestFile, ManifestPackage},
    };

    use super::*;

    #[test]
    fn updates_version_with_spec_prefix_and_double_quotes() {
        let gemspec = Gemspec::new();
        let content = r#"Gem::Specification.new do |spec|
  spec.name = "my-gem"
  spec.version = "1.0.0"
end
"#;

        let manifest = ManifestFile {
            path: Path::new("my-gem.gemspec").to_path_buf(),
            basename: "my-gem.gemspec".to_string(),
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

        let result = gemspec.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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
            path: Path::new("my-gem.gemspec").to_path_buf(),
            basename: "my-gem.gemspec".to_string(),
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

        let result = gemspec.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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
            path: Path::new("my-gem.gemspec").to_path_buf(),
            basename: "my-gem.gemspec".to_string(),
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

        let result = gemspec.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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
            path: Path::new("my-gem.gemspec").to_path_buf(),
            basename: "my-gem.gemspec".to_string(),
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

        let result = gemspec.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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
            path: Path::new("my-gem.gemspec").to_path_buf(),
            basename: "my-gem.gemspec".to_string(),
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

        let result = gemspec.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("spec.version = \"2.0.0\""));
        assert!(updated.contains("spec.name = \"my-gem\""));
        assert!(updated.contains("spec.authors = [\"Test Author\"]"));
        assert!(updated.contains("spec.summary = \"A test gem\""));
        assert!(updated.contains("spec.add_dependency \"rails\", \"~> 7.0\""));
    }

    #[test]
    fn process_packages_returns_none_when_no_gemspec_files() {
        let gemspec = Gemspec::new();

        let manifest = ManifestFile {
            path: Path::new("Gemfile").to_path_buf(),
            basename: "Gemfile".to_string(),
            content: "source 'https://rubygems.org'\ngem 'rails'".to_string(),
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

        let result = gemspec.update(&manifest).unwrap();

        assert!(result.is_none());
    }
}
