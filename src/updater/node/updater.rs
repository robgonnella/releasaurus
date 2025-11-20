use async_trait::async_trait;

use crate::{
    forge::request::FileChange,
    result::Result,
    updater::{framework::UpdaterPackage, traits::PackageUpdater},
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
}

#[async_trait]
impl PackageUpdater for NodeUpdater {
    async fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        if let Some(changes) = self
            .package_json
            .process_package(package, &workspace_packages)
            .await?
        {
            file_changes.extend(changes);
        }

        if let Some(changes) = self
            .package_lock
            .process_package(package, &workspace_packages)
            .await?
        {
            file_changes.extend(changes);
        }

        if let Some(changes) = self
            .yarn_lock
            .process_package(package, &workspace_packages)
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::analyzer::release::Tag;
//     use crate::forge::traits::MockForge;
//     use crate::test_helpers::create_test_updater_package;
//     use crate::updater::framework::Framework;
//     use semver::Version as SemVer;

//     #[tokio::test]
//     async fn test_update_multiple_packages() {
//         let updater = NodeUpdater::new();

//         let packages = vec![
//             create_test_updater_package(
//                 "package-a",
//                 "packages/a",
//                 "2.0.0",
//                 Framework::Node,
//             ),
//             create_test_updater_package(
//                 "package-b",
//                 "packages/b",
//                 "3.0.0",
//                 Framework::Node,
//             ),
//         ];

//         let package_a_json = r#"{
//   "name": "package-a",
//   "version": "1.0.0",
//   "dependencies": {
//     "package-b": "^1.0.0"
//   }
// }"#;

//         let package_b_json = r#"{
//   "name": "package-b",
//   "version": "1.0.0"
// }"#;

//         let mut mock_forge = MockForge::new();

//         // Get package names
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/a/package.json"))
//             .times(1)
//             .returning({
//                 let content = package_a_json.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/b/package.json"))
//             .times(1)
//             .returning({
//                 let content = package_b_json.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         // Update package.json files
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/a/package.json"))
//             .times(1)
//             .returning({
//                 let content = package_a_json.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/b/package.json"))
//             .times(1)
//             .returning({
//                 let content = package_b_json.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         // Check for workspace-level package-lock.json
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("package-lock.json"))
//             .times(1)
//             .returning(|_| Ok(None));

//         // Check for package-level package-lock.json files
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/a/package-lock.json"))
//             .times(1)
//             .returning(|_| Ok(None));

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/b/package-lock.json"))
//             .times(1)
//             .returning(|_| Ok(None));

//         // Check for workspace-level yarn.lock
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("yarn.lock"))
//             .times(1)
//             .returning(|_| Ok(None));

//         // Check for package-level yarn.lock files
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/a/yarn.lock"))
//             .times(1)
//             .returning(|_| Ok(None));

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/b/yarn.lock"))
//             .times(1)
//             .returning(|_| Ok(None));

//         let result = updater.update(packages).await.unwrap();

//         assert!(result.is_some());
//         let changes = result.unwrap();
//         assert_eq!(changes.len(), 2); // 2 package.json files

//         // Check package-a was updated
//         let change_a = changes
//             .iter()
//             .find(|c| c.path == "packages/a/package.json")
//             .unwrap();
//         assert!(change_a.content.contains("\"version\": \"2.0.0\""));
//         assert!(change_a.content.contains("\"package-b\": \"^3.0.0\""));

//         // Check package-b was updated
//         let change_b = changes
//             .iter()
//             .find(|c| c.path == "packages/b/package.json")
//             .unwrap();
//         assert!(change_b.content.contains("\"version\": \"3.0.0\""));
//     }

//     #[tokio::test]
//     async fn test_update_filters_node_packages() {
//         let updater = NodeUpdater::new();

//         let packages = vec![
//             create_test_updater_package(
//                 "node-package",
//                 "packages/node",
//                 "2.0.0",
//                 Framework::Node,
//             ),
//             UpdaterPackage {
//                 name: "java-package".into(),
//                 path: "packages/java".into(),
//                 workspace_root: ".".into(),
//                 framework: Framework::Java,
//                 next_version: Tag {
//                     sha: "test-sha".into(),
//                     name: "v1.0.0".into(),
//                     semver: SemVer::parse("1.0.0").unwrap(),
//                 },
//             },
//         ];

//         let node_json = r#"{
//   "name": "node-package",
//   "version": "1.0.0"
// }"#;

//         let mut mock_forge = MockForge::new();

//         // Get package names
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/node/package.json"))
//             .times(1)
//             .returning({
//                 let content = node_json.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         // Update package.json
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/node/package.json"))
//             .times(1)
//             .returning({
//                 let content = node_json.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         // Check for workspace-level package-lock.json
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("package-lock.json"))
//             .times(1)
//             .returning(|_| Ok(None));

//         // Check for package-level package-lock.json
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/node/package-lock.json"))
//             .times(1)
//             .returning(|_| Ok(None));

//         // Check for workspace-level yarn.lock
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("yarn.lock"))
//             .times(1)
//             .returning(|_| Ok(None));

//         // Check for package-level yarn.lock
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/node/yarn.lock"))
//             .times(1)
//             .returning(|_| Ok(None));

//         let result = updater.update(packages).await.unwrap();

//         assert!(result.is_some());
//         let changes = result.unwrap();
//         assert_eq!(changes.len(), 1);
//         assert_eq!(changes[0].path, "packages/node/package.json");
//     }

//     #[tokio::test]
//     async fn test_get_packages_with_names() {
//         let updater = NodeUpdater::new();

//         let packages = vec![
//             create_test_updater_package(
//                 "pkg-a",
//                 "packages/a",
//                 "2.0.0",
//                 Framework::Node,
//             ),
//             create_test_updater_package(
//                 "pkg-b",
//                 "packages/b",
//                 "3.0.0",
//                 Framework::Node,
//             ),
//         ];

//         let package_a_json = r#"{
//   "name": "actual-name-a",
//   "version": "1.0.0"
// }"#;

//         let package_b_json = r#"{
//   "name": "actual-name-b",
//   "version": "1.0.0"
// }"#;

//         let mut mock_forge = MockForge::new();

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/a/package.json"))
//             .times(1)
//             .returning({
//                 let content = package_a_json.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/b/package.json"))
//             .times(1)
//             .returning({
//                 let content = package_b_json.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         let result = updater.get_packages_with_names(packages).await;

//         assert_eq!(result.len(), 2);
//         assert_eq!(result[0].0, "actual-name-a");
//         assert_eq!(result[1].0, "actual-name-b");
//     }

//     #[tokio::test]
//     async fn test_update_workspace_lock_files() {
//         let updater = NodeUpdater::new();

//         // Create packages with custom workspace_root
//         let mut package_a = create_test_updater_package(
//             "package-a",
//             "packages/a",
//             "2.0.0",
//             Framework::Node,
//         );
//         package_a.workspace_root = "node-workspace".to_string();

//         let mut package_b = create_test_updater_package(
//             "package-b",
//             "packages/b",
//             "3.0.0",
//             Framework::Node,
//         );
//         package_b.workspace_root = "node-workspace".to_string();

//         let packages = vec![package_a, package_b];

//         let package_a_json = r#"{
//   "name": "package-a",
//   "version": "1.0.0",
//   "dependencies": {
//     "package-b": "^1.0.0"
//   }
// }"#;

//         let package_b_json = r#"{
//   "name": "package-b",
//   "version": "1.0.0"
// }"#;

//         let workspace_lock = r#"{
//   "name": "workspace",
//   "version": "1.0.0",
//   "lockfileVersion": 2,
//   "packages": {
//     "node_modules/package-a": {
//       "version": "1.0.0"
//     },
//     "node_modules/package-b": {
//       "version": "1.0.0"
//     }
//   }
// }"#;

//         let mut mock_forge = MockForge::new();

//         // Get package names
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq(
//                 "node-workspace/packages/a/package.json",
//             ))
//             .times(1)
//             .returning({
//                 let content = package_a_json.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq(
//                 "node-workspace/packages/b/package.json",
//             ))
//             .times(1)
//             .returning({
//                 let content = package_b_json.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         // Update package.json files
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq(
//                 "node-workspace/packages/a/package.json",
//             ))
//             .times(1)
//             .returning({
//                 let content = package_a_json.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq(
//                 "node-workspace/packages/b/package.json",
//             ))
//             .times(1)
//             .returning({
//                 let content = package_b_json.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         // Workspace-level package-lock.json
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("node-workspace/package-lock.json"))
//             .times(1)
//             .returning({
//                 let content = workspace_lock.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         // Package-level package-lock.json files (don't exist)
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq(
//                 "node-workspace/packages/a/package-lock.json",
//             ))
//             .times(1)
//             .returning(|_| Ok(None));

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq(
//                 "node-workspace/packages/b/package-lock.json",
//             ))
//             .times(1)
//             .returning(|_| Ok(None));

//         // Workspace-level yarn.lock (doesn't exist)
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("node-workspace/yarn.lock"))
//             .times(1)
//             .returning(|_| Ok(None));

//         // Package-level yarn.lock files (don't exist)
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq(
//                 "node-workspace/packages/a/yarn.lock",
//             ))
//             .times(1)
//             .returning(|_| Ok(None));

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq(
//                 "node-workspace/packages/b/yarn.lock",
//             ))
//             .times(1)
//             .returning(|_| Ok(None));

//         let result = updater.update(packages).await.unwrap();

//         assert!(result.is_some());
//         let changes = result.unwrap();

//         // Should have workspace lock + 2 package.json files = 3 changes
//         assert_eq!(changes.len(), 3);

//         // Check workspace lock was updated
//         let lock_change = changes
//             .iter()
//             .find(|c| c.path == "node-workspace/package-lock.json")
//             .unwrap();
//         assert!(lock_change.content.contains("\"version\": \"2.0.0\""));
//         assert!(lock_change.content.contains("\"version\": \"3.0.0\""));

//         // Check package.json files were updated
//         assert!(
//             changes
//                 .iter()
//                 .any(|c| c.path == "node-workspace/packages/a/package.json")
//         );
//         assert!(
//             changes
//                 .iter()
//                 .any(|c| c.path == "node-workspace/packages/b/package.json")
//         );
//     }
// }
