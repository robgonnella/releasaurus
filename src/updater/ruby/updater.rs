use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        manager::UpdaterPackage,
        ruby::{gemspec::Gemspec, version_rb::VersionRb},
        traits::PackageUpdater,
    },
};

/// Ruby package updater for Gem and Bundler projects.
pub struct RubyUpdater {
    gemspec: Gemspec,
    version_rb: VersionRb,
}

impl RubyUpdater {
    /// Create Ruby updater for Gem and Bundler projects.
    pub fn new() -> Self {
        Self {
            gemspec: Gemspec::new(),
            version_rb: VersionRb::new(),
        }
    }
}

impl PackageUpdater for RubyUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        // workspaces not supported for ruby projects
        _workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        // Try to update gemspec files
        if let Some(changes) = self.gemspec.process_packages(package)? {
            file_changes.extend(changes);
        }

        // Try to update version.rb files
        if let Some(changes) = self.version_rb.process_package(package)? {
            file_changes.extend(changes);
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
    fn processes_ruby_project() {
        let updater = RubyUpdater::new();
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
            manifest_files: vec![manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Ruby,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_some());
        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_ruby_files() {
        let updater = RubyUpdater::new();
        let manifest = ManifestFile {
            is_workspace: false,
            path: "package.json".to_string(),
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
            release_type: ReleaseType::Ruby,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_none());
    }
}
