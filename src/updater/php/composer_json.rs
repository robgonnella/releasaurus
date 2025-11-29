use log::*;
use serde_json::{Value, json};

use crate::{
    cli::Result,
    forge::request::{FileChange, FileUpdateType},
    updater::framework::UpdaterPackage,
};

/// Handles composer.json file parsing and version updates for PHP packages.
pub struct ComposerJson {}

impl ComposerJson {
    /// Create ComposerJson handler for composer.json version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Process composer.json files for all PHP packages.
    pub fn process_package(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename != "composer.json" {
                continue;
            }

            if let Some(mut doc) = self.load_doc(&manifest.content)? {
                info!(
                    "found composer.json for package: {}",
                    manifest.file_path
                );

                // Update the version field
                if let Some(obj) = doc.as_object_mut() {
                    info!(
                        "updating {} version to {}",
                        manifest.file_path, package.next_version.semver
                    );

                    obj.insert(
                        "version".to_string(),
                        json!(package.next_version.semver.to_string()),
                    );

                    let formatted = serde_json::to_string_pretty(&doc)?;

                    file_changes.push(FileChange {
                        path: manifest.file_path.clone(),
                        content: formatted,
                        update_type: FileUpdateType::Replace,
                    });
                } else {
                    warn!(
                        "composer.json is not a valid JSON object: {}",
                        manifest.file_path
                    );
                }
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    /// Load and parse composer.json file from repository into serde_json Value.
    fn load_doc(&self, content: &str) -> Result<Option<Value>> {
        let doc: Value = serde_json::from_str(content)?;
        Ok(Some(doc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ManifestFile,
        test_helpers::create_test_tag,
        updater::framework::{Framework, UpdaterPackage},
    };

    #[tokio::test]
    async fn updates_version_field() {
        let composer_json = ComposerJson::new();
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
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Php,
        };

        let result = composer_json.process_package(&package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
    }

    #[tokio::test]
    async fn inserts_version_field_when_missing() {
        let composer_json = ComposerJson::new();
        let content =
            r#"{"name":"vendor/package","description":"A test package"}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "composer.json".to_string(),
            file_basename: "composer.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "vendor/package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Php,
        };

        let result = composer_json.process_package(&package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
        assert!(updated.contains("\"description\": \"A test package\""));
    }

    #[tokio::test]
    async fn preserves_other_fields() {
        let composer_json = ComposerJson::new();
        let content = r#"{
  "name": "vendor/package",
  "version": "1.0.0",
  "description": "A test package",
  "type": "library",
  "require": {
    "php": "^8.0"
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "composer.json".to_string(),
            file_basename: "composer.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "vendor/package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Php,
        };

        let result = composer_json.process_package(&package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
        assert!(updated.contains("\"name\": \"vendor/package\""));
        assert!(updated.contains("\"description\": \"A test package\""));
        assert!(updated.contains("\"type\": \"library\""));
        assert!(updated.contains("\"php\": \"^8.0\""));
    }

    #[tokio::test]
    async fn process_package_handles_multiple_composer_files() {
        let composer_json = ComposerJson::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            file_path: "packages/a/composer.json".to_string(),
            file_basename: "composer.json".to_string(),
            content: r#"{"name":"vendor/package-a","version":"1.0.0"}"#
                .to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            file_path: "packages/b/composer.json".to_string(),
            file_basename: "composer.json".to_string(),
            content: r#"{"name":"vendor/package-b","version":"1.0.0"}"#
                .to_string(),
        };
        let package = UpdaterPackage {
            package_name: "vendor/package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Php,
        };

        let result = composer_json.process_package(&package).unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[tokio::test]
    async fn process_package_returns_none_when_no_manifest_files() {
        let composer_json = ComposerJson::new();
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Php,
        };

        let result = composer_json.process_package(&package).unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn process_package_returns_none_when_no_composer_json_files() {
        let composer_json = ComposerJson::new();
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "package.json".to_string(),
            file_basename: "package.json".to_string(),
            content: r#"{"name":"my-package","version":"1.0.0"}"#.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Php,
        };

        let result = composer_json.process_package(&package).unwrap();

        assert!(result.is_none());
    }
}
