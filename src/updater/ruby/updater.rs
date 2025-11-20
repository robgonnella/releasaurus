use async_trait::async_trait;

use crate::{
    forge::request::FileChange,
    result::Result,
    updater::{
        framework::UpdaterPackage,
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

#[async_trait]
impl PackageUpdater for RubyUpdater {
    async fn update(
        &self,
        package: &UpdaterPackage,
        // workspaces not supported for ruby projects
        _workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        // Try to update gemspec files
        if let Some(changes) = self.gemspec.process_packages(package).await? {
            file_changes.extend(changes);
        }

        // Try to update version.rb files
        if let Some(changes) = self.version_rb.process_package(package).await? {
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
        test_helpers::create_test_tag,
        updater::framework::{Framework, ManifestFile, UpdaterPackage},
    };

    #[tokio::test]
    async fn processes_ruby_project() {
        let updater = RubyUpdater::new();
        let content = r#"Gem::Specification.new do |spec|
  spec.name = "my-gem"
  spec.version = "1.0.0"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "my-gem.gemspec".to_string(),
            file_basename: "my-gem.gemspec".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = updater.update(&package, vec![]).await.unwrap();

        assert!(result.is_some());
        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[tokio::test]
    async fn returns_none_when_no_ruby_files() {
        let updater = RubyUpdater::new();
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "package.json".to_string(),
            file_basename: "package.json".to_string(),
            content: r#"{"version":"1.0.0"}"#.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = updater.update(&package, vec![]).await.unwrap();

        assert!(result.is_none());
    }
}
