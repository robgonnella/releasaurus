use crate::{
    config::release_type::ReleaseType,
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{
        composite::CompositeUpdater,
        ruby::{gemspec::Gemspec, version_rb::VersionRb},
        traits::FileUpdater,
    },
};

/// Ruby package updater for Gem and Bundler projects.
pub struct RubyUpdater {
    composite: CompositeUpdater,
}

impl RubyUpdater {
    /// Create Ruby updater for Gem and Bundler projects.
    pub fn new() -> Self {
        Self {
            composite: CompositeUpdater::new(vec![
                Box::new(Gemspec::new()),
                Box::new(VersionRb::new()),
            ]),
        }
    }
}

impl Default for RubyUpdater {
    fn default() -> Self {
        RubyUpdater::new()
    }
}

impl FileUpdater for RubyUpdater {
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if !matches!(manifest.release_type, ReleaseType::Ruby) {
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
    };

    use super::*;

    #[test]
    fn processes_ruby_project() {
        let updater = RubyUpdater::new();
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

        let result = updater.update(&manifest).unwrap();

        assert!(result.unwrap().content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_ruby_files() {
        let updater = RubyUpdater::new();

        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".to_string(),
            content: r#"{"version":"1.0.0"}"#.to_string(),
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

        let result = updater.update(&manifest).unwrap();

        assert!(result.is_none());
    }
}
