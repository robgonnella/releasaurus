use regex::Regex;
use std::{path::Path, sync::LazyLock};

use crate::{
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{generic::updater::GenericUpdater, traits::FileUpdater},
};

/// A specification's `version` (`spec.version`, `s.version`, …) assigned a
/// string literal. Whatever the literal holds is replaced; an assignment
/// from a constant (`spec.version = MyGem::VERSION`) has nothing to match,
/// and `required_ruby_version` is a different attribute.
static GEMSPEC_VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^\s*\w+\.version\s*=\s*['"](?<version>[^'"]*)['"]"#)
        .unwrap()
});

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
                &GEMSPEC_VERSION_REGEX,
            ));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::release_type::ReleaseType,
        forge::request::{Tag, TagVersion},
        packages::manifests::{ManifestFile, ManifestPackage},
    };
    use chrono::{TimeZone, Utc};

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
                    version: TagVersion::Semantic(
                        semver::Version::parse("2.0.0").unwrap(),
                    ),
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
                    version: TagVersion::Semantic(
                        semver::Version::parse("2.0.0").unwrap(),
                    ),
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
                    version: TagVersion::Semantic(
                        semver::Version::parse("2.0.0").unwrap(),
                    ),
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
                    version: TagVersion::Semantic(
                        semver::Version::parse("2.0.0").unwrap(),
                    ),
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
                    version: TagVersion::Semantic(
                        semver::Version::parse("2.0.0").unwrap(),
                    ),
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
                    version: TagVersion::Semantic(
                        semver::Version::parse("2.0.0").unwrap(),
                    ),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = gemspec.update(&manifest).unwrap();

        assert!(result.is_none());
    }

    /// Whatever value is set is replaced, whatever its shape.
    #[test]
    fn updates_a_calver_version() {
        let updater = Gemspec::new();
        let manifest = ManifestFile {
            path: Path::new("my-gem.gemspec").to_path_buf(),
            basename: "my-gem.gemspec".to_string(),
            content: "  spec.version = \"26.09\"\n".to_string(),
            release_type: ReleaseType::Ruby,
            owner: Some(ManifestPackage {
                name: "pkg".to_string(),
                release_type: ReleaseType::Ruby,
                tag: Tag {
                    name: "v26.10".into(),
                    version: TagVersion::Strftime(
                        Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap(),
                        "%y.%m".into(),
                    ),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let updated = updater.update(&manifest).unwrap().unwrap().content;

        assert_eq!(updated, "  spec.version = \"26.10\"\n");
    }

    /// Only a literal assigned to the specification's own `version` is
    /// ours: a constant reference and other `*_version` attributes are not.
    #[test]
    fn leaves_non_literal_and_other_version_attributes_alone() {
        let content = r#"Gem::Specification.new do |spec|
  spec.version = MyGem::VERSION
  spec.required_ruby_version = ">= 2.7.0"
  spec.required_rubygems_version = ">= 3.0.0"
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
                    version: TagVersion::Semantic(
                        semver::Version::parse("2.0.0").unwrap(),
                    ),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        assert!(Gemspec::new().update(&manifest).unwrap().is_none());
    }
}
