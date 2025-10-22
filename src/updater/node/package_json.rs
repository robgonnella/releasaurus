use log::*;
use serde_json::{Value, json};

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
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
    pub async fn process_packages(
        &self,
        packages: &[(String, UpdaterPackage)],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for (package_name, package) in packages.iter() {
            let pkg_json = package.get_file_path("package.json");

            let pkg_doc = self.load_doc(&pkg_json, loader).await?;
            if pkg_doc.is_none() {
                continue;
            }

            let mut pkg_doc = pkg_doc.unwrap();
            pkg_doc["version"] = json!(package.next_version.semver.to_string());

            let other_pkgs = packages
                .iter()
                .filter(|(n, _)| n != package_name)
                .cloned()
                .collect::<Vec<(String, UpdaterPackage)>>();

            self.update_deps(&mut pkg_doc, "dependencies", &other_pkgs)?;
            self.update_deps(&mut pkg_doc, "devDependencies", &other_pkgs)?;

            let formatted_json = serde_json::to_string_pretty(&pkg_doc)?;

            file_changes.push(FileChange {
                path: pkg_json,
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
        other_packages: &[(String, UpdaterPackage)],
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

                if let Some((_, package)) =
                    other_packages.iter().find(|(n, _)| n == &dep_name)
                {
                    deps[&dep_name] =
                        json!(format!("^{}", package.next_version.semver));
                }
            }
        }

        Ok(())
    }

    async fn load_doc(
        &self,
        file_path: &str,
        loader: &dyn FileLoader,
    ) -> Result<Option<Value>> {
        let content = loader.get_file_content(file_path).await?;
        if content.is_none() {
            return Ok(None);
        }
        let content = content.unwrap();
        let doc = serde_json::from_str(&content)?;
        Ok(Some(doc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::traits::MockFileLoader;
    use crate::test_helpers::create_test_updater_package;
    use crate::updater::framework::Framework;

    #[tokio::test]
    async fn test_preserves_workspace_protocol_dependencies() {
        let package_json_handler = PackageJson::new();

        let package_a = create_test_updater_package(
            "package-a",
            "packages/a",
            "2.0.0",
            Framework::Node,
        );
        let package_b = create_test_updater_package(
            "package-b",
            "packages/b",
            "3.0.0",
            Framework::Node,
        );

        let package_json_content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "dependencies": {
    "package-b": "workspace:*",
    "external-lib": "^1.0.0"
  }
}"#;

        let package_b_content = r#"{
  "name": "package-b",
  "version": "2.0.0"
}"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/package.json"))
            .times(1)
            .returning(move |_| Ok(Some(package_json_content.to_string())));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/package.json"))
            .times(1)
            .returning(move |_| Ok(Some(package_b_content.to_string())));

        let packages = vec![
            ("package-a".to_string(), package_a),
            ("package-b".to_string(), package_b),
        ];

        let result = package_json_handler
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);

        // Find the change for package-a
        let change_a = changes
            .iter()
            .find(|c| c.path == "packages/a/package.json")
            .unwrap();
        let content = &change_a.content;
        // workspace:* should be preserved
        assert!(content.contains("\"package-b\": \"workspace:*\""));
        // Version should be updated
        assert!(content.contains("\"version\": \"2.0.0\""));
    }

    #[tokio::test]
    async fn test_preserves_repo_protocol_dependencies() {
        let package_json_handler = PackageJson::new();

        let package_a = create_test_updater_package(
            "package-a",
            "packages/a",
            "2.0.0",
            Framework::Node,
        );
        let package_b = create_test_updater_package(
            "package-b",
            "packages/b",
            "3.0.0",
            Framework::Node,
        );

        let package_json_content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "dependencies": {
    "package-b": "repo:packages/b"
  },
  "devDependencies": {
    "package-b": "repo:*"
  }
}"#;

        let package_b_content = r#"{
  "name": "package-b",
  "version": "2.0.0"
}"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/package.json"))
            .times(1)
            .returning(move |_| Ok(Some(package_json_content.to_string())));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/package.json"))
            .times(1)
            .returning(move |_| Ok(Some(package_b_content.to_string())));

        let packages = vec![
            ("package-a".to_string(), package_a),
            ("package-b".to_string(), package_b),
        ];

        let result = package_json_handler
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);

        // Find the change for package-a
        let change_a = changes
            .iter()
            .find(|c| c.path == "packages/a/package.json")
            .unwrap();
        let content = &change_a.content;
        // repo: protocols should be preserved
        assert!(content.contains("\"package-b\": \"repo:packages/b\""));
        assert!(content.contains("\"package-b\": \"repo:*\""));
    }

    #[tokio::test]
    async fn test_updates_normal_dependencies_while_preserving_workspace() {
        let package_json_handler = PackageJson::new();

        let package_a = create_test_updater_package(
            "package-a",
            "packages/a",
            "2.0.0",
            Framework::Node,
        );
        let package_b = create_test_updater_package(
            "package-b",
            "packages/b",
            "3.0.0",
            Framework::Node,
        );
        let package_c = create_test_updater_package(
            "package-c",
            "packages/c",
            "4.0.0",
            Framework::Node,
        );

        let package_json_content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "dependencies": {
    "package-b": "workspace:*",
    "package-c": "^1.0.0"
  }
}"#;

        let package_b_content = r#"{
  "name": "package-b",
  "version": "2.0.0"
}"#;

        let package_c_content = r#"{
  "name": "package-c",
  "version": "3.0.0"
}"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/package.json"))
            .times(1)
            .returning(move |_| Ok(Some(package_json_content.to_string())));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/package.json"))
            .times(1)
            .returning(move |_| Ok(Some(package_b_content.to_string())));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/c/package.json"))
            .times(1)
            .returning(move |_| Ok(Some(package_c_content.to_string())));

        let packages = vec![
            ("package-a".to_string(), package_a),
            ("package-b".to_string(), package_b),
            ("package-c".to_string(), package_c),
        ];

        let result = package_json_handler
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 3);

        // Find the change for package-a
        let change_a = changes
            .iter()
            .find(|c| c.path == "packages/a/package.json")
            .unwrap();
        let content = &change_a.content;
        // workspace:* should be preserved
        assert!(content.contains("\"package-b\": \"workspace:*\""));
        // Normal dependency should be updated
        assert!(content.contains("\"package-c\": \"^4.0.0\""));
        // Version should be updated
        assert!(content.contains("\"version\": \"2.0.0\""));
    }
}
