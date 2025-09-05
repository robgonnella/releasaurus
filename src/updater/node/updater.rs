use color_eyre::eyre::Result;
use log::*;
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use crate::updater::framework::{Framework, Package};
use crate::updater::traits::PackageUpdater;

/// Node.js package updater supporting npm, yarn, and pnpm
pub struct NodeUpdater {}

impl NodeUpdater {
    pub fn new() -> Self {
        Self {}
    }

    fn load_doc<P: AsRef<Path>>(&self, file_path: P) -> Result<Value> {
        let file = OpenOptions::new().read(true).open(file_path)?;
        let doc: Value = serde_json::from_reader(file)?;
        Ok(doc)
    }

    fn write_doc<P: AsRef<Path>>(
        &self,
        doc: &Value,
        file_path: P,
    ) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(file_path)?;
        file.write_all(doc.to_string().as_bytes())?;
        Ok(())
    }

    fn update_deps(
        &self,
        doc: &mut Value,
        dep_kind: &str,
        other_packages: &[(String, Package)],
    ) -> Result<()> {
        if let Some(deps) = doc[dep_kind].as_object_mut() {
            for (key, value) in deps {
                if let Some((_, other_package)) =
                    other_packages.iter().find(|(n, _)| n == key)
                {
                    *value =
                        json!(other_package.next_version.semver.to_string());
                }
            }
        }

        Ok(())
    }

    fn get_packages_with_names(
        &self,
        packages: Vec<Package>,
    ) -> Vec<(String, Package)> {
        packages
            .into_iter()
            .map(|p| {
                let manifest_path = Path::new(&p.path).join("package.json");
                if let Ok(doc) = self.load_doc(manifest_path)
                    && let Some(name) = doc["name"].as_str()
                {
                    return (name.to_string(), p);
                }
                (p.name.clone(), p)
            })
            .collect::<Vec<(String, Package)>>()
    }
}

impl PackageUpdater for NodeUpdater {
    fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()> {
        let node_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Node))
            .collect::<Vec<Package>>();

        info!(
            "Found {} node packages in {}",
            node_packages.len(),
            root_path.display(),
        );

        let packages_with_names = self.get_packages_with_names(node_packages);

        for (package_name, package) in packages_with_names.iter() {
            let pkg_json = Path::new(&package.path).join("package.json");
            let mut pkg_doc = self.load_doc(pkg_json.as_path())?;
            pkg_doc["version"] = json!(package.next_version.semver.to_string());

            let other_pkgs = packages_with_names
                .iter()
                .filter(|(n, _)| n != package_name)
                .cloned()
                .collect::<Vec<(String, Package)>>();

            self.update_deps(&mut pkg_doc, "dependencies", &other_pkgs)?;
            self.update_deps(&mut pkg_doc, "dev_dependencies", &other_pkgs)?;
            self.write_doc(&pkg_doc, pkg_json.as_path())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::types::Version;
    use crate::updater::framework::Framework;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_version(version: &str) -> Version {
        Version {
            tag: format!("v{}", version),
            semver: semver::Version::parse(version).unwrap(),
        }
    }

    fn create_test_package(name: &str, path: &str, version: &str) -> Package {
        Package::new(
            name.to_string(),
            path.to_string(),
            create_test_version(version),
            Framework::Node,
        )
    }

    #[test]
    fn test_load_doc_success() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        fs::write(
            &package_json,
            r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "express": "^4.17.1"
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let doc = updater.load_doc(&package_json).unwrap();

        assert_eq!(doc["name"].as_str(), Some("test-package"));
        assert_eq!(doc["version"].as_str(), Some("1.0.0"));
        assert_eq!(doc["dependencies"]["express"].as_str(), Some("^4.17.1"));
    }

    #[test]
    fn test_load_doc_file_not_found() {
        let updater = NodeUpdater::new();
        let result = updater.load_doc("/nonexistent/package.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_doc_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        fs::write(&package_json, "invalid json content").unwrap();

        let updater = NodeUpdater::new();
        let result = updater.load_doc(&package_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_doc() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        // Create initial file
        fs::write(
            &package_json,
            r#"{
  "name": "test-package",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let mut doc = updater.load_doc(&package_json).unwrap();

        // Modify the document
        doc["version"] = json!("2.0.0");

        // Write it back
        updater.write_doc(&doc, &package_json).unwrap();

        // Verify the change
        let updated_content = fs::read_to_string(&package_json).unwrap();
        assert!(updated_content.contains("\"version\":\"2.0.0\""));
    }

    #[test]
    fn test_update_deps_dependencies() {
        let updater = NodeUpdater::new();
        let mut doc = json!({
            "name": "test-package",
            "version": "1.0.0",
            "dependencies": {
                "my-dep": "1.0.0",
                "external-dep": "2.0.0"
            }
        });

        let other_packages = vec![(
            "my-dep".to_string(),
            create_test_package("my-dep", "/path/to/my-dep", "1.5.0"),
        )];

        updater
            .update_deps(&mut doc, "dependencies", &other_packages)
            .unwrap();

        assert_eq!(doc["dependencies"]["my-dep"].as_str(), Some("1.5.0"));
        assert_eq!(doc["dependencies"]["external-dep"].as_str(), Some("2.0.0"));
    }

    #[test]
    fn test_update_deps_dev_dependencies() {
        let updater = NodeUpdater::new();
        let mut doc = json!({
            "name": "test-package",
            "version": "1.0.0",
            "dev_dependencies": {
                "test-dep": "1.0.0"
            }
        });

        let other_packages = vec![(
            "test-dep".to_string(),
            create_test_package("test-dep", "/path/to/test-dep", "2.1.0"),
        )];

        updater
            .update_deps(&mut doc, "dev_dependencies", &other_packages)
            .unwrap();

        assert_eq!(doc["dev_dependencies"]["test-dep"].as_str(), Some("2.1.0"));
    }

    #[test]
    fn test_update_deps_no_dependencies() {
        let updater = NodeUpdater::new();
        let mut doc = json!({
            "name": "test-package",
            "version": "1.0.0"
        });

        let other_packages = vec![(
            "my-dep".to_string(),
            create_test_package("my-dep", "/path/to/my-dep", "1.5.0"),
        )];

        // Should not error when dependencies section doesn't exist
        updater
            .update_deps(&mut doc, "dependencies", &other_packages)
            .unwrap();

        assert!(doc["dependencies"].is_null());
    }

    #[test]
    fn test_get_packages_with_names_from_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("my-package");
        fs::create_dir_all(&pkg_path).unwrap();

        fs::write(
            pkg_path.join("package.json"),
            r#"{
  "name": "@scope/actual-name",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "my-package",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        let packages_with_names = updater.get_packages_with_names(packages);

        assert_eq!(packages_with_names.len(), 1);
        assert_eq!(packages_with_names[0].0, "@scope/actual-name");
        assert_eq!(packages_with_names[0].1.name, "my-package");
    }

    #[test]
    fn test_get_packages_with_names_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("my-package");
        fs::create_dir_all(&pkg_path).unwrap();

        // Create invalid package.json or missing file
        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "my-package",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        let packages_with_names = updater.get_packages_with_names(packages);

        assert_eq!(packages_with_names.len(), 1);
        assert_eq!(packages_with_names[0].0, "my-package");
    }

    #[test]
    fn test_update_single_package() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("my-package");
        fs::create_dir_all(&package_path).unwrap();

        let package_json = package_path.join("package.json");
        fs::write(
            &package_json,
            r#"{
  "name": "my-package",
  "version": "1.0.0",
  "dependencies": {
    "express": "^4.17.1"
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "my-package",
            package_path.to_str().unwrap(),
            "2.0.0",
        )];

        updater.update(temp_dir.path(), packages).unwrap();

        let updated_content = fs::read_to_string(&package_json).unwrap();
        assert!(updated_content.contains("\"version\":\"2.0.0\""));
        // Express dependency should remain unchanged
        assert!(updated_content.contains("\"express\":\"^4.17.1\""));
    }

    #[test]
    fn test_update_cross_package_dependencies() {
        let temp_dir = TempDir::new().unwrap();

        // Create package A
        let pkg_a_path = temp_dir.path().join("pkg-a");
        fs::create_dir_all(&pkg_a_path).unwrap();
        fs::write(
            pkg_a_path.join("package.json"),
            r#"{
  "name": "pkg-a",
  "version": "1.0.0",
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#,
        )
        .unwrap();

        // Create package B that depends on A
        let pkg_b_path = temp_dir.path().join("pkg-b");
        fs::create_dir_all(&pkg_b_path).unwrap();
        fs::write(
            pkg_b_path.join("package.json"),
            r#"{
  "name": "pkg-b",
  "version": "1.0.0",
  "dependencies": {
    "pkg-a": "1.0.0",
    "express": "^4.17.1"
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![
            create_test_package("pkg-a", pkg_a_path.to_str().unwrap(), "2.0.0"),
            create_test_package("pkg-b", pkg_b_path.to_str().unwrap(), "1.1.0"),
        ];

        updater.update(temp_dir.path(), packages).unwrap();

        // Check that pkg-a was updated to 2.0.0
        let pkg_a_content =
            fs::read_to_string(pkg_a_path.join("package.json")).unwrap();
        assert!(pkg_a_content.contains("\"version\":\"2.0.0\""));

        // Check that pkg-b was updated to 1.1.0 and its dependency on pkg-a was updated
        let pkg_b_content =
            fs::read_to_string(pkg_b_path.join("package.json")).unwrap();
        assert!(pkg_b_content.contains("\"version\":\"1.1.0\""));
        assert!(pkg_b_content.contains("\"pkg-a\":\"2.0.0\""));
        // External dependency should remain unchanged
        assert!(pkg_b_content.contains("\"express\":\"^4.17.1\""));
    }

    #[test]
    fn test_update_with_dev_dependencies() {
        let temp_dir = TempDir::new().unwrap();

        let pkg_a_path = temp_dir.path().join("pkg-a");
        let pkg_b_path = temp_dir.path().join("pkg-b");
        fs::create_dir_all(&pkg_a_path).unwrap();
        fs::create_dir_all(&pkg_b_path).unwrap();

        fs::write(
            pkg_a_path.join("package.json"),
            r#"{
  "name": "pkg-a",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        fs::write(
            pkg_b_path.join("package.json"),
            r#"{
  "name": "pkg-b",
  "version": "1.0.0",
  "dev_dependencies": {
    "pkg-a": "1.0.0",
    "jest": "^27.0.0"
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![
            create_test_package("pkg-a", pkg_a_path.to_str().unwrap(), "1.5.0"),
            create_test_package("pkg-b", pkg_b_path.to_str().unwrap(), "1.2.0"),
        ];

        updater.update(temp_dir.path(), packages).unwrap();

        let pkg_b_content =
            fs::read_to_string(pkg_b_path.join("package.json")).unwrap();
        assert!(pkg_b_content.contains("\"version\":\"1.2.0\""));
        assert!(pkg_b_content.contains("\"pkg-a\":\"1.5.0\""));
        assert!(pkg_b_content.contains("\"jest\":\"^27.0.0\""));
    }

    #[test]
    fn test_update_filters_non_node_packages() {
        let temp_dir = TempDir::new().unwrap();

        let pkg_path = temp_dir.path().join("pkg");
        fs::create_dir_all(&pkg_path).unwrap();
        fs::write(
            pkg_path.join("package.json"),
            r#"{
  "name": "pkg",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![
            create_test_package("pkg", pkg_path.to_str().unwrap(), "2.0.0"),
            Package::new(
                "rust-pkg".to_string(),
                pkg_path.to_str().unwrap().to_string(),
                create_test_version("1.1.0"),
                Framework::Rust,
            ),
        ];

        updater.update(temp_dir.path(), packages).unwrap();

        // Only the Node package should be updated
        let content =
            fs::read_to_string(pkg_path.join("package.json")).unwrap();
        assert!(content.contains("\"version\":\"2.0.0\""));
    }

    #[test]
    fn test_update_with_scoped_package_names() {
        let temp_dir = TempDir::new().unwrap();

        let pkg_a_path = temp_dir.path().join("pkg-a");
        let pkg_b_path = temp_dir.path().join("pkg-b");
        fs::create_dir_all(&pkg_a_path).unwrap();
        fs::create_dir_all(&pkg_b_path).unwrap();

        fs::write(
            pkg_a_path.join("package.json"),
            r#"{
  "name": "@myorg/pkg-a",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        fs::write(
            pkg_b_path.join("package.json"),
            r#"{
  "name": "@myorg/pkg-b",
  "version": "1.0.0",
  "dependencies": {
    "@myorg/pkg-a": "1.0.0"
  }
}"#,
        )
        .unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![
            create_test_package("pkg-a", pkg_a_path.to_str().unwrap(), "2.0.0"),
            create_test_package("pkg-b", pkg_b_path.to_str().unwrap(), "1.5.0"),
        ];

        updater.update(temp_dir.path(), packages).unwrap();

        let pkg_b_content =
            fs::read_to_string(pkg_b_path.join("package.json")).unwrap();
        assert!(pkg_b_content.contains("\"version\":\"1.5.0\""));
        assert!(pkg_b_content.contains("\"@myorg/pkg-a\":\"2.0.0\""));
    }

    #[test]
    fn test_update_with_missing_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_path = temp_dir.path().join("pkg");
        fs::create_dir_all(&pkg_path).unwrap();

        let updater = NodeUpdater::new();
        let packages = vec![create_test_package(
            "pkg",
            pkg_path.to_str().unwrap(),
            "2.0.0",
        )];

        let result = updater.update(temp_dir.path(), packages);
        assert!(result.is_err());
    }
}
