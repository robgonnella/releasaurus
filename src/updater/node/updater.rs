use async_trait::async_trait;
use log::*;
use serde_json::Value;

use crate::{
    forge::{request::FileChange, traits::FileLoader},
    result::Result,
    updater::{
        framework::{Framework, UpdaterPackage},
        traits::PackageUpdater,
    },
};

use super::package_json::PackageJson;
use super::package_lock::PackageLock;
use super::yarn_lock::YarnLock;

/// Node.js package updater for npm, yarn, and pnpm projects.
pub struct NodeUpdater {
    package_json: PackageJson,
    package_lock: PackageLock,
    yarn_lock: YarnLock,
}

impl NodeUpdater {
    /// Create Node.js updater for package.json and lock file management.
    pub fn new() -> Self {
        Self {
            package_json: PackageJson::new(),
            package_lock: PackageLock::new(),
            yarn_lock: YarnLock::new(),
        }
    }

    /// Load and parse JSON file from repository into serde_json Value.
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
        let doc: Value = serde_json::from_str(&content)?;
        Ok(Some(doc))
    }

    /// Extract package names from package.json files and pair with Package structs.
    async fn get_packages_with_names(
        &self,
        packages: Vec<UpdaterPackage>,
        loader: &dyn FileLoader,
    ) -> Vec<(String, UpdaterPackage)> {
        let results = packages.into_iter().map(|p| async {
            let manifest_path = p.get_file_path("package.json");
            let content = self.load_doc(&manifest_path, loader).await;
            if let Ok(content) = content
                && let Some(doc) = content
                && let Some(name) = doc["name"].as_str()
            {
                return (name.to_string(), p);
            }

            (p.name.clone(), p)
        });

        futures::future::join_all(results).await
    }
}

#[async_trait]
impl PackageUpdater for NodeUpdater {
    async fn update(
        &self,
        packages: Vec<UpdaterPackage>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let node_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Node))
            .collect::<Vec<UpdaterPackage>>();

        info!("Found {} node packages", node_packages.len());

        if node_packages.is_empty() {
            return Ok(None);
        }

        let mut file_changes: Vec<FileChange> = vec![];

        let packages_with_names =
            self.get_packages_with_names(node_packages, loader).await;

        // Update package.json files
        if let Some(changes) = self
            .package_json
            .process_packages(&packages_with_names, loader)
            .await?
        {
            file_changes.extend(changes);
        }

        // Update package-lock.json files
        if let Some(changes) = self
            .package_lock
            .process_packages(&packages_with_names, loader)
            .await?
        {
            file_changes.extend(changes);
        }

        // Update yarn.lock files
        if let Some(changes) = self
            .yarn_lock
            .process_packages(&packages_with_names, loader)
            .await?
        {
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
    use crate::analyzer::release::Tag;
    use crate::forge::traits::MockFileLoader;
    use crate::test_helpers::create_test_updater_package;
    use semver::Version as SemVer;

    #[tokio::test]
    async fn test_update_multiple_packages() {
        let updater = NodeUpdater::new();

        let packages = vec![
            create_test_updater_package(
                "package-a",
                "packages/a",
                "2.0.0",
                Framework::Node,
            ),
            create_test_updater_package(
                "package-b",
                "packages/b",
                "3.0.0",
                Framework::Node,
            ),
        ];

        let package_a_json = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "dependencies": {
    "package-b": "^1.0.0"
  }
}"#;

        let package_b_json = r#"{
  "name": "package-b",
  "version": "1.0.0"
}"#;

        let mut mock_loader = MockFileLoader::new();

        // Get package names
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/package.json"))
            .times(1)
            .returning({
                let content = package_a_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/package.json"))
            .times(1)
            .returning({
                let content = package_b_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Update package.json files
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/package.json"))
            .times(1)
            .returning({
                let content = package_a_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/package.json"))
            .times(1)
            .returning({
                let content = package_b_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Check for workspace-level package-lock.json
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("package-lock.json"))
            .times(1)
            .returning(|_| Ok(None));

        // Check for package-level package-lock.json files
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/package-lock.json"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/package-lock.json"))
            .times(1)
            .returning(|_| Ok(None));

        // Check for workspace-level yarn.lock
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("yarn.lock"))
            .times(1)
            .returning(|_| Ok(None));

        // Check for package-level yarn.lock files
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/yarn.lock"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/yarn.lock"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2); // 2 package.json files

        // Check package-a was updated
        let change_a = changes
            .iter()
            .find(|c| c.path == "packages/a/package.json")
            .unwrap();
        assert!(change_a.content.contains("\"version\": \"2.0.0\""));
        assert!(change_a.content.contains("\"package-b\": \"^3.0.0\""));

        // Check package-b was updated
        let change_b = changes
            .iter()
            .find(|c| c.path == "packages/b/package.json")
            .unwrap();
        assert!(change_b.content.contains("\"version\": \"3.0.0\""));
    }

    #[tokio::test]
    async fn test_update_filters_node_packages() {
        let updater = NodeUpdater::new();

        let packages = vec![
            create_test_updater_package(
                "node-package",
                "packages/node",
                "2.0.0",
                Framework::Node,
            ),
            UpdaterPackage {
                name: "java-package".into(),
                path: "packages/java".into(),
                workspace_root: ".".into(),
                framework: Framework::Java,
                next_version: Tag {
                    sha: "test-sha".into(),
                    name: "v1.0.0".into(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                },
            },
        ];

        let node_json = r#"{
  "name": "node-package",
  "version": "1.0.0"
}"#;

        let mut mock_loader = MockFileLoader::new();

        // Get package names
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/node/package.json"))
            .times(1)
            .returning({
                let content = node_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Update package.json
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/node/package.json"))
            .times(1)
            .returning({
                let content = node_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Check for workspace-level package-lock.json
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("package-lock.json"))
            .times(1)
            .returning(|_| Ok(None));

        // Check for package-level package-lock.json
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/node/package-lock.json"))
            .times(1)
            .returning(|_| Ok(None));

        // Check for workspace-level yarn.lock
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("yarn.lock"))
            .times(1)
            .returning(|_| Ok(None));

        // Check for package-level yarn.lock
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/node/yarn.lock"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/node/package.json");
    }

    #[tokio::test]
    async fn test_get_packages_with_names() {
        let updater = NodeUpdater::new();

        let packages = vec![
            create_test_updater_package(
                "pkg-a",
                "packages/a",
                "2.0.0",
                Framework::Node,
            ),
            create_test_updater_package(
                "pkg-b",
                "packages/b",
                "3.0.0",
                Framework::Node,
            ),
        ];

        let package_a_json = r#"{
  "name": "actual-name-a",
  "version": "1.0.0"
}"#;

        let package_b_json = r#"{
  "name": "actual-name-b",
  "version": "1.0.0"
}"#;

        let mut mock_loader = MockFileLoader::new();

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/a/package.json"))
            .times(1)
            .returning({
                let content = package_a_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/b/package.json"))
            .times(1)
            .returning({
                let content = package_b_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let result = updater
            .get_packages_with_names(packages, &mock_loader)
            .await;

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "actual-name-a");
        assert_eq!(result[1].0, "actual-name-b");
    }

    #[tokio::test]
    async fn test_update_workspace_lock_files() {
        let updater = NodeUpdater::new();

        // Create packages with custom workspace_root
        let mut package_a = create_test_updater_package(
            "package-a",
            "packages/a",
            "2.0.0",
            Framework::Node,
        );
        package_a.workspace_root = "node-workspace".to_string();

        let mut package_b = create_test_updater_package(
            "package-b",
            "packages/b",
            "3.0.0",
            Framework::Node,
        );
        package_b.workspace_root = "node-workspace".to_string();

        let packages = vec![package_a, package_b];

        let package_a_json = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "dependencies": {
    "package-b": "^1.0.0"
  }
}"#;

        let package_b_json = r#"{
  "name": "package-b",
  "version": "1.0.0"
}"#;

        let workspace_lock = r#"{
  "name": "workspace",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "packages": {
    "node_modules/package-a": {
      "version": "1.0.0"
    },
    "node_modules/package-b": {
      "version": "1.0.0"
    }
  }
}"#;

        let mut mock_loader = MockFileLoader::new();

        // Get package names
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "node-workspace/packages/a/package.json",
            ))
            .times(1)
            .returning({
                let content = package_a_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "node-workspace/packages/b/package.json",
            ))
            .times(1)
            .returning({
                let content = package_b_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Update package.json files
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "node-workspace/packages/a/package.json",
            ))
            .times(1)
            .returning({
                let content = package_a_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "node-workspace/packages/b/package.json",
            ))
            .times(1)
            .returning({
                let content = package_b_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Workspace-level package-lock.json
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("node-workspace/package-lock.json"))
            .times(1)
            .returning({
                let content = workspace_lock.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Package-level package-lock.json files (don't exist)
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "node-workspace/packages/a/package-lock.json",
            ))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "node-workspace/packages/b/package-lock.json",
            ))
            .times(1)
            .returning(|_| Ok(None));

        // Workspace-level yarn.lock (doesn't exist)
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("node-workspace/yarn.lock"))
            .times(1)
            .returning(|_| Ok(None));

        // Package-level yarn.lock files (don't exist)
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "node-workspace/packages/a/yarn.lock",
            ))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "node-workspace/packages/b/yarn.lock",
            ))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();

        // Should have workspace lock + 2 package.json files = 3 changes
        assert_eq!(changes.len(), 3);

        // Check workspace lock was updated
        let lock_change = changes
            .iter()
            .find(|c| c.path == "node-workspace/package-lock.json")
            .unwrap();
        assert!(lock_change.content.contains("\"version\": \"2.0.0\""));
        assert!(lock_change.content.contains("\"version\": \"3.0.0\""));

        // Check package.json files were updated
        assert!(
            changes
                .iter()
                .any(|c| c.path == "node-workspace/packages/a/package.json")
        );
        assert!(
            changes
                .iter()
                .any(|c| c.path == "node-workspace/packages/b/package.json")
        );
    }
}
