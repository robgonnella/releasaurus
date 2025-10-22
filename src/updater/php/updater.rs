use async_trait::async_trait;
use log::*;

use crate::{
    forge::{request::FileChange, traits::FileLoader},
    result::Result,
    updater::{
        framework::{Framework, UpdaterPackage},
        php::composer_json::ComposerJson,
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

        self.composer_json
            .process_packages(&php_packages, loader)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::release::Tag;
    use crate::forge::traits::MockFileLoader;
    use crate::test_helpers::create_test_updater_package;
    use semver::Version as SemVer;

    #[tokio::test]
    async fn test_update_filters_php_packages() {
        let updater = PhpUpdater::new();

        let packages = vec![
            create_test_updater_package(
                "php-package",
                "packages/php",
                "2.0.0",
                Framework::Php,
            ),
            UpdaterPackage {
                name: "node-package".to_string(),
                path: "packages/node".to_string(),
                workspace_root: ".".into(),
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
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "3.0.0",
            Framework::Php,
        );

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
        assert!(content.contains("3.0.0"));
        assert!(content.contains("vendor/package"));
        assert!(content.contains("library"));
        assert!(content.contains("symfony/console"));
        assert!(content.contains("^6.0"));
    }

    #[tokio::test]
    async fn test_update_multiple_packages() {
        let updater = PhpUpdater::new();
        let packages = vec![
            create_test_updater_package(
                "package-one",
                "packages/one",
                "2.0.0",
                Framework::Php,
            ),
            create_test_updater_package(
                "package-two",
                "packages/two",
                "3.0.0",
                Framework::Php,
            ),
        ];

        let composer1 = r#"{
  "name": "vendor/package-one",
  "version": "1.0.0"
}"#;

        let composer2 = r#"{
  "name": "vendor/package-two",
  "version": "1.0.0"
}"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/one/composer.json"))
            .times(1)
            .returning({
                let content = composer1.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/two/composer.json"))
            .times(1)
            .returning({
                let content = composer2.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);

        // Check first package
        let change1 = changes
            .iter()
            .find(|c| c.path == "packages/one/composer.json")
            .unwrap();
        assert!(change1.content.contains("2.0.0"));

        // Check second package
        let change2 = changes
            .iter()
            .find(|c| c.path == "packages/two/composer.json")
            .unwrap();
        assert!(change2.content.contains("3.0.0"));
    }
}
