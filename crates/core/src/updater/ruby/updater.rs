use crate::{
    forge::request::FileChange,
    result::Result,
    updater::{
        composite::CompositeUpdater,
        manager::UpdaterPackage,
        ruby::{gemspec::Gemspec, version_rb::VersionRb},
        traits::PackageUpdater,
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

impl PackageUpdater for RubyUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        self.composite.update(package, workspace_packages)
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, rc::Rc};

    use crate::{
        config::release_type::ReleaseType, forge::request::Tag,
        packages::manifests::ManifestFile, updater::dispatch::Updater,
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
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            manifest_files: vec![manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Ruby)),
        };

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_ruby_files() {
        let updater = RubyUpdater::new();
        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".to_string(),
            content: r#"{"version":"1.0.0"}"#.to_string(),
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

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }
}
