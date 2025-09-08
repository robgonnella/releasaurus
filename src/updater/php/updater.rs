use log::*;
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;

use crate::{
    result::Result,
    updater::framework::Framework,
    updater::{framework::Package, traits::PackageUpdater},
};

pub struct PhpUpdater {}

impl PhpUpdater {
    pub fn new() -> Self {
        Self {}
    }

    /// Load a composer.json file as a JSON Value
    fn load_doc<P: AsRef<Path>>(&self, file_path: P) -> Result<Value> {
        let mut file = OpenOptions::new().read(true).open(file_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let doc: Value = serde_json::from_str(&content)?;
        Ok(doc)
    }

    /// Write a JSON Value back to a composer.json file with pretty formatting
    fn write_doc<P: AsRef<Path>>(
        &self,
        doc: &Value,
        file_path: P,
    ) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(file_path)?;
        let formatted_json = serde_json::to_string_pretty(doc)?;
        file.write_all(formatted_json.as_bytes())?;
        Ok(())
    }

    /// Process packages and update their composer.json files
    fn process_packages(&self, packages: &[Package]) -> Result<()> {
        for package in packages {
            let file_path = Path::new(&package.path).join("composer.json");

            if !file_path.exists() {
                info!(
                    "skipping: no composer.json detected for package: {}",
                    package.path
                );
                continue;
            }

            info!("found composer.json for package: {}", package.path);

            let mut doc = self.load_doc(&file_path)?;

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

                self.write_doc(&doc, &file_path)?;
            } else {
                warn!(
                    "composer.json is not a valid JSON object: {}",
                    file_path.display()
                );
            }
        }

        Ok(())
    }
}

impl PackageUpdater for PhpUpdater {
    fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()> {
        let php_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Php))
            .collect::<Vec<Package>>();

        info!(
            "Found {} PHP packages in {}",
            php_packages.len(),
            root_path.display(),
        );

        self.process_packages(&php_packages)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyzer::types::Version, updater::framework::Framework};
    use std::fs;
    use tempfile::TempDir;

    fn create_test_package(
        name: &str,
        path: &str,
        version: &str,
        framework: Framework,
    ) -> Package {
        Package::new(
            name.to_string(),
            path.to_string(),
            Version {
                tag: format!("v{}", version),
                semver: semver::Version::parse(version).unwrap(),
            },
            framework,
        )
    }

    #[test]
    fn test_php_updater_creation() {
        let _updater = PhpUpdater::new();
        // Basic test to ensure the updater can be created without panicking
    }

    #[test]
    fn test_php_updater_empty_packages() {
        let updater = PhpUpdater::new();
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let packages = vec![];

        let result = updater.update(path, packages);
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_filters_php_packages_only() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create PHP package
        let php_dir = root_path.join("php-package");
        fs::create_dir_all(&php_dir).unwrap();
        fs::write(
            php_dir.join("composer.json"),
            r#"{
    "name": "test/php-package",
    "version": "1.0.0",
    "require": {
        "php": ">=8.0"
    }
}"#,
        )
        .unwrap();

        // Create non-PHP package
        let node_dir = root_path.join("node-package");
        fs::create_dir_all(&node_dir).unwrap();

        let packages = vec![
            create_test_package(
                "test/php-package",
                php_dir.to_str().unwrap(),
                "2.0.0",
                Framework::Php,
            ),
            create_test_package(
                "node-package",
                node_dir.to_str().unwrap(),
                "2.0.0",
                Framework::Node,
            ),
        ];

        let updater = PhpUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Check that PHP package was updated
        let updated_content =
            fs::read_to_string(php_dir.join("composer.json")).unwrap();
        assert!(updated_content.contains("\"version\": \"2.0.0\""));
    }

    #[test]
    fn test_update_single_php_package() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("test-package");
        fs::create_dir_all(&package_dir).unwrap();

        // Create initial composer.json
        fs::write(
            package_dir.join("composer.json"),
            r#"{
    "name": "vendor/test-package",
    "version": "1.0.0",
    "type": "library",
    "require": {
        "php": ">=8.0"
    },
    "autoload": {
        "psr-4": {
            "Vendor\\TestPackage\\": "src/"
        }
    }
}"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "vendor/test-package",
            package_dir.to_str().unwrap(),
            "2.1.0",
            Framework::Php,
        )];

        let updater = PhpUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Verify the version was updated
        let updated_content =
            fs::read_to_string(package_dir.join("composer.json")).unwrap();
        assert!(updated_content.contains("\"version\": \"2.1.0\""));

        // Verify other fields remain unchanged
        assert!(updated_content.contains("\"name\": \"vendor/test-package\""));
        assert!(updated_content.contains("\"type\": \"library\""));
        assert!(updated_content.contains("\"php\": \">=8.0\""));
    }

    #[test]
    fn test_update_multiple_php_packages() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create first package
        let package1_dir = root_path.join("package1");
        fs::create_dir_all(&package1_dir).unwrap();
        fs::write(
            package1_dir.join("composer.json"),
            r#"{
    "name": "vendor/package1",
    "version": "1.0.0",
    "require": {
        "php": ">=8.0"
    }
}"#,
        )
        .unwrap();

        // Create second package
        let package2_dir = root_path.join("package2");
        fs::create_dir_all(&package2_dir).unwrap();
        fs::write(
            package2_dir.join("composer.json"),
            r#"{
    "name": "vendor/package2",
    "version": "0.5.0",
    "require": {
        "php": ">=8.1"
    }
}"#,
        )
        .unwrap();

        let packages = vec![
            create_test_package(
                "vendor/package1",
                package1_dir.to_str().unwrap(),
                "1.1.0",
                Framework::Php,
            ),
            create_test_package(
                "vendor/package2",
                package2_dir.to_str().unwrap(),
                "0.6.0",
                Framework::Php,
            ),
        ];

        let updater = PhpUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Verify both packages were updated
        let package1_content =
            fs::read_to_string(package1_dir.join("composer.json")).unwrap();
        assert!(package1_content.contains("\"version\": \"1.1.0\""));

        let package2_content =
            fs::read_to_string(package2_dir.join("composer.json")).unwrap();
        assert!(package2_content.contains("\"version\": \"0.6.0\""));
    }

    #[test]
    fn test_update_with_missing_composer_json() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create package directory without composer.json
        let package_dir = root_path.join("no-composer");
        fs::create_dir_all(&package_dir).unwrap();

        let packages = vec![create_test_package(
            "no-composer",
            package_dir.to_str().unwrap(),
            "1.0.0",
            Framework::Php,
        )];

        let updater = PhpUpdater::new();
        let result = updater.update(root_path, packages);
        // Should succeed but skip the package
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_with_malformed_composer_json() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("malformed");
        fs::create_dir_all(&package_dir).unwrap();

        // Create malformed composer.json
        fs::write(
            package_dir.join("composer.json"),
            r#"{ "name": "test", invalid json }"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "malformed",
            package_dir.to_str().unwrap(),
            "1.0.0",
            Framework::Php,
        )];

        let updater = PhpUpdater::new();
        let result = updater.update(root_path, packages);
        // Should return an error due to malformed JSON
        assert!(result.is_err());
    }

    #[test]
    fn test_load_and_write_doc() {
        let temp_dir = TempDir::new().unwrap();
        let composer_json = temp_dir.path().join("composer.json");

        // Create initial file
        fs::write(
            &composer_json,
            r#"{
    "name": "test/package",
    "version": "1.0.0",
    "require": {
        "php": ">=8.0"
    }
}"#,
        )
        .unwrap();

        let updater = PhpUpdater::new();
        let mut doc = updater.load_doc(&composer_json).unwrap();

        // Modify the document
        if let Some(obj) = doc.as_object_mut() {
            obj.insert("version".to_string(), json!("2.0.0"));
        }

        // Write it back
        updater.write_doc(&doc, &composer_json).unwrap();

        // Verify the change
        let updated_content = fs::read_to_string(&composer_json).unwrap();
        assert!(updated_content.contains("\"version\": \"2.0.0\""));
    }

    #[test]
    fn test_update_preserves_json_structure() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("structured");
        fs::create_dir_all(&package_dir).unwrap();

        // Create composer.json with complex structure
        fs::write(
            package_dir.join("composer.json"),
            r#"{
    "name": "vendor/structured-package",
    "version": "1.0.0",
    "type": "library",
    "description": "A test package",
    "keywords": ["test", "example"],
    "license": "MIT",
    "authors": [
        {
            "name": "Test Author",
            "email": "test@example.com"
        }
    ],
    "require": {
        "php": ">=8.0",
        "symfony/console": "^6.0"
    },
    "require-dev": {
        "phpunit/phpunit": "^10.0"
    },
    "autoload": {
        "psr-4": {
            "Vendor\\StructuredPackage\\": "src/"
        }
    }
}"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "vendor/structured-package",
            package_dir.to_str().unwrap(),
            "1.2.3",
            Framework::Php,
        )];

        let updater = PhpUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_dir.join("composer.json")).unwrap();

        // Verify version was updated
        assert!(updated_content.contains("\"version\": \"1.2.3\""));

        // Verify other structure is preserved
        assert!(
            updated_content.contains("\"name\": \"vendor/structured-package\"")
        );
        assert!(updated_content.contains("\"type\": \"library\""));
        assert!(
            updated_content.contains("\"description\": \"A test package\"")
        );
        assert!(updated_content.contains("\"license\": \"MIT\""));
        assert!(updated_content.contains("\"symfony/console\": \"^6.0\""));
        assert!(updated_content.contains("\"phpunit/phpunit\": \"^10.0\""));
        assert!(updated_content.contains("\"psr-4\""));
    }

    #[test]
    fn test_update_composer_json_without_version() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("no-version");
        fs::create_dir_all(&package_dir).unwrap();

        // Create composer.json without version field
        fs::write(
            package_dir.join("composer.json"),
            r#"{
    "name": "vendor/no-version-package",
    "type": "library",
    "require": {
        "php": ">=8.0"
    }
}"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "vendor/no-version-package",
            package_dir.to_str().unwrap(),
            "1.0.0",
            Framework::Php,
        )];

        let updater = PhpUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Verify version was added
        let updated_content =
            fs::read_to_string(package_dir.join("composer.json")).unwrap();
        assert!(updated_content.contains("\"version\": \"1.0.0\""));
        assert!(
            updated_content.contains("\"name\": \"vendor/no-version-package\"")
        );
    }
}
