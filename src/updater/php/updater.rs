use async_trait::async_trait;
use log::*;
use serde_json::{Value, json};
use std::path::Path;

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
    updater::{
        framework::{Framework, UpdaterPackage},
        traits::PackageUpdater,
    },
};

/// PHP package updater for Composer projects.
pub struct PhpUpdater {}

impl PhpUpdater {
    /// Create PHP updater for Composer composer.json files.
    pub fn new() -> Self {
        Self {}
    }

    /// Load and parse composer.json file from repository into serde_json
    /// Value.
    async fn load_doc<P: AsRef<Path>>(
        &self,
        file_path: P,
        loader: &dyn FileLoader,
    ) -> Result<Option<Value>> {
        let file_path = file_path.as_ref().display().to_string();
        let content = loader.get_file_content(&file_path).await?;
        if content.is_none() {
            return Ok(None);
        }
        let content = content.unwrap();
        let doc: Value = serde_json::from_str(&content)?;
        Ok(Some(doc))
    }

    /// Update version numbers in composer.json files for PHP packages using
    /// Composer.
    async fn process_packages(
        &self,
        packages: &[UpdaterPackage],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];
        for package in packages {
            let file_path = Path::new(&package.path).join("composer.json");

            let doc = self.load_doc(&file_path, loader).await?;

            if doc.is_none() {
                continue;
            }

            info!("found composer.json for package: {}", package.path);
            let mut doc = doc.unwrap();

            // Update the version field
            if let Some(obj) = doc.as_object_mut() {
                info!(
                    "updating {} version to {}",
                    file_path.display(),
                    package.next_version.semver
                );

                obj.insert(
                    "version".to_string(),
                    json!(package.next_version.semver.to_string()),
                );

                let formatted = serde_json::to_string_pretty(&doc)?;

                file_changes.push(FileChange {
                    path: file_path.display().to_string(),
                    content: formatted,
                    update_type: FileUpdateType::Replace,
                });
            } else {
                warn!(
                    "composer.json is not a valid JSON object: {}",
                    file_path.display()
                );
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}

#[async_trait]
impl PackageUpdater for PhpUpdater {
    async fn update(
        &self,
        packages: Vec<UpdaterPackage>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let php_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Php))
            .collect::<Vec<UpdaterPackage>>();

        info!("Found {} PHP packages", php_packages.len());

        if php_packages.is_empty() {
            return Ok(None);
        }

        self.process_packages(&php_packages, loader).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::release::Tag;
    use crate::forge::traits::MockFileLoader;
    use semver::Version as SemVer;

    fn create_test_package(
        name: &str,
        path: &str,
        next_version: &str,
    ) -> UpdaterPackage {
        UpdaterPackage {
            name: name.to_string(),
            path: path.to_string(),
            framework: Framework::Php,
            next_version: Tag {
                sha: "test-sha".to_string(),
                name: format!("v{}", next_version),
                semver: SemVer::parse(next_version).unwrap(),
            },
        }
    }

    #[tokio::test]
    async fn test_load_doc() {
        let updater = PhpUpdater::new();
        let composer_json = r#"{
  "name": "test/package",
  "version": "1.0.0",
  "description": "A test package"
}"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-package/composer.json"))
            .times(1)
            .returning(move |_| Ok(Some(composer_json.to_string())));

        let result = updater
            .load_doc("test-package/composer.json", &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let doc = result.unwrap();
        assert_eq!(doc["name"], "test/package");
        assert_eq!(doc["version"], "1.0.0");
        assert_eq!(doc["description"], "A test package");
    }

    #[tokio::test]
    async fn test_load_doc_file_not_found() {
        let updater = PhpUpdater::new();

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-package/composer.json"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater
            .load_doc("test-package/composer.json", &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_process_packages_single_package() {
        let updater = PhpUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let composer_json = r#"{
  "name": "test/package",
  "version": "1.0.0",
  "description": "A test package",
  "require": {
    "php": "^8.0"
  }
}"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/composer.json"))
            .times(1)
            .returning(move |_| Ok(Some(composer_json.to_string())));

        let packages = vec![package];
        let result = updater
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/composer.json");
        assert!(changes[0].content.contains("\"version\": \"2.0.0\""));
        assert!(changes[0].content.contains("\"name\": \"test/package\""));
        assert!(changes[0].content.contains("\"php\": \"^8.0\""));
    }

    #[tokio::test]
    async fn test_process_packages_multiple_packages() {
        let updater = PhpUpdater::new();
        let packages = vec![
            create_test_package("package-one", "packages/one", "2.0.0"),
            create_test_package("package-two", "packages/two", "3.0.0"),
        ];

        let composer1_json = r#"{
  "name": "test/package-one",
  "version": "1.0.0"
}"#;

        let composer2_json = r#"{
  "name": "test/package-two",
  "version": "1.0.0"
}"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/one/composer.json"))
            .times(1)
            .returning({
                let content = composer1_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/two/composer.json"))
            .times(1)
            .returning({
                let content = composer2_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let result = updater
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);

        // Check first package
        let change1 = changes
            .iter()
            .find(|c| c.path == "packages/one/composer.json")
            .unwrap();
        assert!(change1.content.contains("\"version\": \"2.0.0\""));

        // Check second package
        let change2 = changes
            .iter()
            .find(|c| c.path == "packages/two/composer.json")
            .unwrap();
        assert!(change2.content.contains("\"version\": \"3.0.0\""));
    }

    #[tokio::test]
    async fn test_process_packages_composer_not_found() {
        let updater = PhpUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/composer.json"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_process_packages_no_packages() {
        let updater = PhpUpdater::new();

        let mock_loader = MockFileLoader::new();
        // No expectations since no packages to process

        let packages = vec![];
        let result = updater
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_filters_php_packages() {
        let updater = PhpUpdater::new();

        let packages = vec![
            create_test_package("php-package", "packages/php", "2.0.0"),
            UpdaterPackage {
                name: "node-package".to_string(),
                path: "packages/node".to_string(),
                framework: Framework::Node,
                next_version: Tag {
                    sha: "test-sha".to_string(),
                    name: "v1.0.0".to_string(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                },
            },
        ];

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/php/composer.json"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        // Should return None when no composer.json files are found
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_with_valid_composer_json() {
        let updater = PhpUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "3.0.0");

        let composer_json = r#"{
  "name": "vendor/package",
  "version": "1.0.0",
  "type": "library",
  "require": {
    "php": "^8.0",
    "symfony/console": "^6.0"
  },
  "autoload": {
    "psr-4": {
      "Vendor\\Package\\": "src/"
    }
  }
}"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/composer.json"))
            .times(1)
            .returning(move |_| Ok(Some(composer_json.to_string())));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/composer.json");

        let content = &changes[0].content;
        assert!(content.contains("\"version\": \"3.0.0\""));
        assert!(content.contains("\"name\": \"vendor/package\""));
        assert!(content.contains("\"type\": \"library\""));
        assert!(content.contains("\"symfony/console\": \"^6.0\""));
    }

    #[tokio::test]
    async fn test_update_preserves_composer_json_structure() {
        let updater = PhpUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.5.0");

        let composer_json = r#"{
  "name": "test/package",
  "version": "1.0.0",
  "description": "Test description",
  "keywords": ["test", "example"],
  "license": "MIT",
  "authors": [
    {
      "name": "John Doe",
      "email": "john@example.com"
    }
  ]
}"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/composer.json"))
            .times(1)
            .returning(move |_| Ok(Some(composer_json.to_string())));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        let content = &changes[0].content;

        // Verify version is updated
        assert!(content.contains("\"version\": \"2.5.0\""));

        // Verify all other fields are preserved
        assert!(content.contains("\"name\": \"test/package\""));
        assert!(content.contains("\"description\": \"Test description\""));
        assert!(content.contains("\"keywords\""));
        assert!(content.contains("\"license\": \"MIT\""));
        assert!(content.contains("\"authors\""));
        assert!(content.contains("\"name\": \"John Doe\""));
    }

    #[tokio::test]
    async fn test_update_adds_version_if_missing() {
        let updater = PhpUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "1.0.0");

        // composer.json without a version field
        let composer_json = r#"{
  "name": "test/package",
  "description": "A test package without version"
}"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/composer.json"))
            .times(1)
            .returning(move |_| Ok(Some(composer_json.to_string())));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);

        let content = &changes[0].content;
        assert!(content.contains("\"version\": \"1.0.0\""));
        assert!(content.contains("\"name\": \"test/package\""));
    }

    #[tokio::test]
    async fn test_process_packages_mixed_found_and_not_found() {
        let updater = PhpUpdater::new();
        let packages = vec![
            create_test_package("package-one", "packages/one", "2.0.0"),
            create_test_package("package-two", "packages/two", "3.0.0"),
        ];

        let composer1_json = r#"{
  "name": "test/package-one",
  "version": "1.0.0"
}"#;

        let mut mock_loader = MockFileLoader::new();

        // First package has composer.json
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/one/composer.json"))
            .times(1)
            .returning({
                let content = composer1_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Second package doesn't have composer.json
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/two/composer.json"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/one/composer.json");
        assert!(changes[0].content.contains("\"version\": \"2.0.0\""));
    }
}
