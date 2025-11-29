use log::*;
use serde_json::{Value, json};

use crate::{
    cli::Result,
    forge::request::{FileChange, FileUpdateType},
    updater::framework::UpdaterPackage,
};

/// Handles package.json file parsing and version updates for Node.js packages.
pub struct PackageJson {}

impl PackageJson {
    /// Create package.json handler for version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version fields in package.json files for all Node packages.
    pub fn process_package(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename != "package.json" {
                continue;
            }

            let mut doc = self.load_doc(&manifest.content)?;
            doc["version"] = json!(package.next_version.semver.to_string());

            let other_pkgs = workspace_packages
                .iter()
                .filter(|p| p.package_name != package.package_name)
                .cloned()
                .collect::<Vec<UpdaterPackage>>();

            self.update_deps(&mut doc, "dependencies", &other_pkgs)?;
            self.update_deps(&mut doc, "devDependencies", &other_pkgs)?;

            let formatted_json = serde_json::to_string_pretty(&doc)?;

            file_changes.push(FileChange {
                path: manifest.file_path.clone(),
                content: formatted_json,
                update_type: FileUpdateType::Replace,
            });
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    fn update_deps(
        &self,
        doc: &mut Value,
        dep_type: &str,
        other_packages: &[UpdaterPackage],
    ) -> Result<()> {
        if doc.get(dep_type).is_none() {
            return Ok(());
        }

        // Skip if this is a workspace package
        if let Some(workspaces) = doc.get("workspaces")
            && (workspaces.is_array() || workspaces.is_object())
        {
            debug!("skipping workspace package.json");
            return Ok(());
        }

        if let Some(deps) = doc[dep_type].as_object_mut() {
            for (dep_name, dep_value) in deps.clone() {
                // Skip workspace: and repo: protocol dependencies
                if let Some(version_str) = dep_value.as_str()
                    && (version_str.starts_with("workspace:")
                        || version_str.starts_with("repo:"))
                {
                    continue;
                }

                if let Some(package) =
                    other_packages.iter().find(|p| p.package_name == dep_name)
                {
                    deps[&dep_name] =
                        json!(format!("^{}", package.next_version.semver));
                }
            }
        }

        Ok(())
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
        config::ManifestFile,
        test_helpers::create_test_tag,
        updater::framework::{Framework, UpdaterPackage},
    };

    #[tokio::test]
    async fn updates_version_field() {
        let package_json = PackageJson::new();
        let content = r#"{"name":"my-package","version":"1.0.0"}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "package.json".to_string(),
            file_basename: "package.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = package_json.process_package(&package, &[]).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
    }

    #[tokio::test]
    async fn updates_dependencies_to_workspace_packages() {
        let package_json = PackageJson::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "dependencies": {
    "package-b": "^1.0.0"
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "packages/a/package.json".to_string(),
            file_basename: "package.json".to_string(),
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

        let result = package_json
            .process_package(&package_a, &[package_a.clone(), package_b])
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"package-b\": \"^3.0.0\""));
    }

    #[tokio::test]
    async fn updates_dev_dependencies_to_workspace_packages() {
        let package_json = PackageJson::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "devDependencies": {
    "package-b": "^1.0.0"
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "packages/a/package.json".to_string(),
            file_basename: "package.json".to_string(),
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

        let result = package_json
            .process_package(&package_a, &[package_a.clone(), package_b])
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"package-b\": \"^3.0.0\""));
    }

    #[tokio::test]
    async fn skips_workspace_protocol_dependencies() {
        let package_json = PackageJson::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "dependencies": {
    "package-b": "workspace:^1.0.0"
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "packages/a/package.json".to_string(),
            file_basename: "package.json".to_string(),
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

        let result = package_json
            .process_package(&package_a, &[package_a.clone(), package_b])
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"package-b\": \"workspace:^1.0.0\""));
    }

    #[tokio::test]
    async fn skips_repo_protocol_dependencies() {
        let package_json = PackageJson::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "dependencies": {
    "package-b": "repo:^1.0.0"
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "packages/a/package.json".to_string(),
            file_basename: "package.json".to_string(),
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

        let result = package_json
            .process_package(&package_a, &[package_a.clone(), package_b])
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"package-b\": \"repo:^1.0.0\""));
    }

    #[tokio::test]
    async fn skips_workspace_root_package_json() {
        let package_json = PackageJson::new();
        let content = r#"{
  "name": "monorepo",
  "version": "1.0.0",
  "workspaces": ["packages/*"],
  "dependencies": {
    "package-a": "^1.0.0"
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "package.json".to_string(),
            file_basename: "package.json".to_string(),
            content: content.to_string(),
        };
        let package_root = UpdaterPackage {
            package_name: "monorepo".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };
        let package_a = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: "packages/a".to_string(),
            manifest_files: vec![],
            next_version: create_test_tag("v3.0.0", "3.0.0", "def"),
            framework: Framework::Node,
        };

        let result = package_json
            .process_package(&package_root, &[package_root.clone(), package_a])
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"package-a\": \"^1.0.0\""));
    }

    #[tokio::test]
    async fn process_package_handles_multiple_package_json_files() {
        let package_json = PackageJson::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            file_path: "packages/a/package.json".to_string(),
            file_basename: "package.json".to_string(),
            content: r#"{"name":"package-a","version":"1.0.0"}"#.to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            file_path: "packages/a/subdir/package.json".to_string(),
            file_basename: "package.json".to_string(),
            content: r#"{"name":"package-a-sub","version":"1.0.0"}"#
                .to_string(),
        };
        let package = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: "packages/a".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = package_json.process_package(&package, &[]).unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[tokio::test]
    async fn process_package_returns_none_when_no_package_json_files() {
        let package_json = PackageJson::new();
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "Cargo.toml".to_string(),
            file_basename: "Cargo.toml".to_string(),
            content: "[package]\nversion = \"1.0.0\"".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = package_json.process_package(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn preserves_other_fields_in_package_json() {
        let package_json = PackageJson::new();
        let content = r#"{
  "name": "my-package",
  "version": "1.0.0",
  "description": "A test package",
  "main": "index.js",
  "scripts": {
    "test": "jest"
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "package.json".to_string(),
            file_basename: "package.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = package_json.process_package(&package, &[]).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
        assert!(updated.contains("\"description\": \"A test package\""));
        assert!(updated.contains("\"main\": \"index.js\""));
        assert!(updated.contains("\"test\": \"jest\""));
    }
}
