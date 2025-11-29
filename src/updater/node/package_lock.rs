use serde_json::{Value, json};

use crate::{
    cli::Result,
    config::ManifestFile,
    forge::request::{FileChange, FileUpdateType},
    updater::framework::UpdaterPackage,
};

/// Handles package-lock.json file parsing and version updates for Node.js packages.
pub struct PackageLock {}

impl PackageLock {
    /// Create package-lock.json handler for version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version fields in package-lock.json files for all Node packages.
    pub fn process_package(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename != "package-lock.json" {
                continue;
            }

            if manifest.is_workspace
                && let Some(change) = self.update_lock_file(
                    manifest,
                    package,
                    workspace_packages,
                )?
            {
                file_changes.push(change);
            } else if let Some(change) =
                self.update_lock_file(manifest, package, &[])?
            {
                file_changes.push(change);
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    /// Update a single package-lock.json file
    fn update_lock_file(
        &self,
        manifest: &ManifestFile,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<FileChange>> {
        let mut lock_doc = self.load_doc(&manifest.content)?;
        lock_doc["version"] = json!(package.next_version.semver.to_string());

        // Update packages section
        if let Some(packages) = lock_doc.get_mut("packages")
            && let Some(packages_obj) = packages.as_object_mut()
        {
            for (key, package_info) in packages_obj {
                if key.is_empty() {
                    // Root package entry - update version for current package
                    package_info["version"] =
                        json!(package.next_version.semver.to_string());

                    // Update dependencies within root package entry
                    if let Some(deps) = package_info.get_mut("dependencies")
                        && let Some(deps_obj) = deps.as_object_mut()
                    {
                        for ws_package in workspace_packages.iter() {
                            if let Some((_, dep_info)) =
                                deps_obj.iter_mut().find(|(name, _)| {
                                    name.to_string() == ws_package.package_name
                                })
                            {
                                *dep_info = json!(format!(
                                    "{}",
                                    ws_package.next_version.semver.to_string()
                                ));
                            }
                        }
                    }

                    // Update devDependencies within root package entry
                    if let Some(deps) = package_info.get_mut("devDependencies")
                        && let Some(deps_obj) = deps.as_object_mut()
                    {
                        for ws_package in workspace_packages.iter() {
                            if let Some((_, dep_info)) =
                                deps_obj.iter_mut().find(|(name, _)| {
                                    name.to_string() == ws_package.package_name
                                })
                            {
                                *dep_info = json!(format!(
                                    "{}",
                                    ws_package.next_version.semver.to_string()
                                ));
                            }
                        }
                    }

                    continue;
                }

                // Extract package name from node_modules/ key
                if let Some(package_name) = key.strip_prefix("node_modules/")
                    && let Some(ws_pkg) = workspace_packages
                        .iter()
                        .find(|p| p.package_name == package_name)
                {
                    package_info["version"] =
                        json!(ws_pkg.next_version.semver.to_string());
                }
            }
        }

        let formatted_json = serde_json::to_string_pretty(&lock_doc)?;

        Ok(Some(FileChange {
            path: manifest.file_path.clone(),
            content: formatted_json,
            update_type: FileUpdateType::Replace,
        }))
    }

    fn load_doc(&self, content: &str) -> Result<Value> {
        let doc = serde_json::from_str(content)?;
        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_helpers::create_test_tag,
        updater::framework::{Framework, UpdaterPackage},
    };

    #[tokio::test]
    async fn updates_version_field() {
        let package_lock = PackageLock::new();
        let content =
            r#"{"name":"my-package","version":"1.0.0","packages":{}}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "package-lock.json".to_string(),
            file_basename: "package-lock.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = package_lock.process_package(&package, &[]).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
    }

    #[tokio::test]
    async fn updates_root_package_entry_version() {
        let package_lock = PackageLock::new();
        let content = r#"{
  "name": "my-package",
  "version": "1.0.0",
  "packages": {
    "": {
      "name": "my-package",
      "version": "1.0.0"
    }
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "package-lock.json".to_string(),
            file_basename: "package-lock.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = package_lock.process_package(&package, &[]).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
        // Should appear twice: once at root, once in packages[""]
        assert_eq!(updated.matches("\"version\": \"2.0.0\"").count(), 2);
    }

    #[tokio::test]
    async fn updates_workspace_dependencies_in_lock_file() {
        let package_lock = PackageLock::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "packages": {
    "": {
      "name": "package-a",
      "version": "1.0.0",
      "dependencies": {
        "package-b": "1.0.0"
      }
    }
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: true,
            file_path: "package-lock.json".to_string(),
            file_basename: "package-lock.json".to_string(),
            content: content.to_string(),
        };
        let package_a = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: "packages/a".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };
        let package_b = UpdaterPackage {
            package_name: "package-b".to_string(),
            workspace_root: "packages/b".to_string(),
            manifest_files: vec![],
            next_version: create_test_tag("v3.0.0", "3.0.0", "def"),
            framework: Framework::Node,
        };

        let result = package_lock
            .process_package(&package_a, &[package_a.clone(), package_b])
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"package-b\": \"3.0.0\""));
    }

    #[tokio::test]
    async fn updates_workspace_dev_dependencies_in_lock_file() {
        let package_lock = PackageLock::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "packages": {
    "": {
      "name": "package-a",
      "version": "1.0.0",
      "devDependencies": {
        "package-b": "1.0.0"
      }
    }
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: true,
            file_path: "package-lock.json".to_string(),
            file_basename: "package-lock.json".to_string(),
            content: content.to_string(),
        };
        let package_a = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: "packages/a".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };
        let package_b = UpdaterPackage {
            package_name: "package-b".to_string(),
            workspace_root: "packages/b".to_string(),
            manifest_files: vec![],
            next_version: create_test_tag("v3.0.0", "3.0.0", "def"),
            framework: Framework::Node,
        };

        let result = package_lock
            .process_package(&package_a, &[package_a.clone(), package_b])
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"package-b\": \"3.0.0\""));
    }

    #[tokio::test]
    async fn updates_node_modules_entries_for_workspace_packages() {
        let package_lock = PackageLock::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "packages": {
    "": {
      "name": "package-a",
      "version": "1.0.0"
    },
    "node_modules/package-b": {
      "version": "1.0.0"
    }
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: true,
            file_path: "package-lock.json".to_string(),
            file_basename: "package-lock.json".to_string(),
            content: content.to_string(),
        };
        let package_a = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: "packages/a".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };
        let package_b = UpdaterPackage {
            package_name: "package-b".to_string(),
            workspace_root: "packages/b".to_string(),
            manifest_files: vec![],
            next_version: create_test_tag("v3.0.0", "3.0.0", "def"),
            framework: Framework::Node,
        };

        let result = package_lock
            .process_package(&package_a, &[package_a.clone(), package_b])
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        let parsed: Value = serde_json::from_str(&updated).unwrap();
        assert_eq!(
            parsed["packages"]["node_modules/package-b"]["version"],
            "3.0.0"
        );
    }

    #[tokio::test]
    async fn handles_non_workspace_lock_files() {
        let package_lock = PackageLock::new();
        let content = r#"{
  "name": "my-package",
  "version": "1.0.0",
  "packages": {
    "": {
      "name": "my-package",
      "version": "1.0.0"
    }
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "package-lock.json".to_string(),
            file_basename: "package-lock.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = package_lock.process_package(&package, &[]).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
    }

    #[tokio::test]
    async fn process_package_handles_multiple_lock_files() {
        let package_lock = PackageLock::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            file_path: "packages/a/package-lock.json".to_string(),
            file_basename: "package-lock.json".to_string(),
            content: r#"{"name":"package-a","version":"1.0.0","packages":{}}"#
                .to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            file_path: "packages/b/package-lock.json".to_string(),
            file_basename: "package-lock.json".to_string(),
            content: r#"{"name":"package-b","version":"1.0.0","packages":{}}"#
                .to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = package_lock.process_package(&package, &[]).unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[tokio::test]
    async fn process_package_returns_none_when_no_lock_files() {
        let package_lock = PackageLock::new();
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
            framework: Framework::Node,
        };

        let result = package_lock.process_package(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn preserves_other_fields_in_lock_file() {
        let package_lock = PackageLock::new();
        let content = r#"{
  "name": "my-package",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "requires": true,
  "packages": {
    "": {
      "name": "my-package",
      "version": "1.0.0"
    }
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "package-lock.json".to_string(),
            file_basename: "package-lock.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = package_lock.process_package(&package, &[]).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
        assert!(updated.contains("\"lockfileVersion\": 2"));
        assert!(updated.contains("\"requires\": true"));
    }
}
