use async_trait::async_trait;

use crate::{
    forge::request::FileChange,
    result::Result,
    updater::{
        framework::UpdaterPackage, php::composer_json::ComposerJson,
        traits::PackageUpdater,
    },
};

/// PHP package updater for Composer projects.
pub struct PhpUpdater {
    composer_json: ComposerJson,
}

impl PhpUpdater {
    /// Create PHP updater for Composer composer.json files.
    pub fn new() -> Self {
        Self {
            composer_json: ComposerJson::new(),
        }
    }
}

#[async_trait]
impl PackageUpdater for PhpUpdater {
    async fn update(
        &self,
        package: &UpdaterPackage,
        // workspaces not supported for php projects
        _workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        self.composer_json.process_package(package).await
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
    async fn processes_php_project() {
        let updater = PhpUpdater::new();
        let content = r#"{"name":"vendor/package","version":"1.0.0"}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "composer.json".to_string(),
            file_basename: "composer.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "vendor/package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Php,
        };

        let result = updater.update(&package, vec![]).await.unwrap();

        assert!(result.is_some());
        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[tokio::test]
    async fn returns_none_when_no_php_files() {
        let updater = PhpUpdater::new();
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
            framework: Framework::Php,
        };

        let result = updater.update(&package, vec![]).await.unwrap();

        assert!(result.is_none());
    }
}
