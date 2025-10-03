use async_trait::async_trait;
use log::*;
use regex::Regex;
use serde_json::{Value, json};
use std::path::Path;

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
    updater::{
        framework::{Framework, Package},
        traits::PackageUpdater,
    },
};

/// Node.js package updater for npm, yarn, and pnpm projects.
pub struct NodeUpdater {}

impl NodeUpdater {
    pub fn new() -> Self {
        Self {}
    }

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
                    // Skip workspace dependencies
                    if let Some(version_str) = value.as_str()
                        && version_str.starts_with("workspace:")
                    {
                        continue;
                    }

                    *value =
                        json!(other_package.next_version.semver.to_string());
                }
            }
        }

        Ok(())
    }

    async fn get_packages_with_names(
        &self,
        packages: Vec<Package>,
        loader: &dyn FileLoader,
    ) -> Vec<(String, Package)> {
        let results = packages.into_iter().map(|p| async {
            let manifest_path = Path::new(&p.path).join("package.json");
            let content = self.load_doc(manifest_path, loader).await;
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

    /// Update package-lock.json file for a specific package
    async fn update_package_lock_json_for_package(
        &self,
        current_package: (&str, &Package),
        other_packages: &[(String, Package)],
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        let lock_path =
            Path::new(&current_package.1.path).join("package-lock.json");

        let lock_doc = self.load_doc(lock_path.clone(), loader).await?;

        if lock_doc.is_none() {
            return Ok(None);
        }

        info!("Updating package-lock.json at {}", lock_path.display());
        let mut lock_doc = lock_doc.unwrap();

        // Get root package name for later use
        let root_name = lock_doc
            .get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        // Update root level version if this lock file corresponds to the
        // current package
        if let Some(ref name) = root_name
            && current_package.0 == name
        {
            lock_doc["version"] =
                json!(current_package.1.next_version.semver.to_string());
        }

        // Update packages section
        if let Some(packages) = lock_doc.get_mut("packages")
            && let Some(packages_obj) = packages.as_object_mut()
        {
            for (key, package_info) in packages_obj {
                if key.is_empty() {
                    // Root package entry - update version if this corresponds
                    // to the current package
                    if let Some(ref name) = root_name
                        && current_package.0 == name
                    {
                        package_info["version"] = json!(
                            current_package.1.next_version.semver.to_string()
                        );
                    }

                    // Update dependencies within root package entry
                    if let Some(deps) = package_info.get_mut("dependencies")
                        && let Some(deps_obj) = deps.as_object_mut()
                    {
                        for (dep_name, dep_info) in deps_obj {
                            if current_package.0 == dep_name {
                                *dep_info = json!(format!(
                                    "^{}",
                                    current_package
                                        .1
                                        .next_version
                                        .semver
                                        .to_string()
                                ));
                            } else if let Some((_, package)) = other_packages
                                .iter()
                                .find(|(n, _)| n == dep_name)
                            {
                                *dep_info = json!(format!(
                                    "^{}",
                                    package.next_version.semver.to_string()
                                ));
                            }
                        }
                    }

                    // Update devDependencies within root package entry
                    if let Some(dev_deps) =
                        package_info.get_mut("devDependencies")
                        && let Some(dev_deps_obj) = dev_deps.as_object_mut()
                    {
                        for (dep_name, dep_info) in dev_deps_obj {
                            if current_package.0 == dep_name {
                                *dep_info = json!(format!(
                                    "^{}",
                                    current_package
                                        .1
                                        .next_version
                                        .semver
                                        .to_string()
                                ));
                            } else if let Some((_, package)) = other_packages
                                .iter()
                                .find(|(n, _)| n == dep_name)
                            {
                                *dep_info = json!(format!(
                                    "^{}",
                                    package.next_version.semver.to_string()
                                ));
                            }
                        }
                    }
                    continue;
                }

                // Extract package name from node_modules/ key
                if let Some(package_name) = key.strip_prefix("node_modules/") {
                    // Check if it's the current package
                    if current_package.0 == package_name {
                        package_info["version"] = json!(
                            current_package.1.next_version.semver.to_string()
                        );
                    }
                    // Check if it's one of the other packages
                    else if let Some((_, package)) =
                        other_packages.iter().find(|(n, _)| n == package_name)
                    {
                        package_info["version"] =
                            json!(package.next_version.semver.to_string());
                    }
                }
            }
        }

        let formatted_json = serde_json::to_string_pretty(&lock_doc)?;

        Ok(Some(FileChange {
            path: lock_path.display().to_string(),
            content: formatted_json,
            update_type: crate::forge::request::FileUpdateType::Replace,
        }))
    }

    /// Update package-lock.json file at root path
    async fn update_package_lock_json_for_root(
        &self,
        root_path: &Path,
        all_packages: &[(String, Package)],
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        let lock_path = root_path.join("package-lock.json");
        let lock_doc = self.load_doc(lock_path.clone(), loader).await?;
        if lock_doc.is_none() {
            return Ok(None);
        }
        info!("Updating package-lock.json at {}", lock_path.display());
        let mut lock_doc = lock_doc.unwrap();

        // Get root package name for later use
        let root_name = lock_doc
            .get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        // Update root level version if this lock file corresponds to one of
        // our packages
        if let Some(ref name) = root_name
            && let Some((_, package)) =
                all_packages.iter().find(|(n, _)| n == name)
        {
            lock_doc["version"] =
                json!(package.next_version.semver.to_string());
        }

        // Update packages section
        if let Some(packages) = lock_doc.get_mut("packages")
            && let Some(packages_obj) = packages.as_object_mut()
        {
            for (key, package_info) in packages_obj {
                if key.is_empty() {
                    // Root package entry - update version if this corresponds
                    // to one of our packages
                    if let Some(ref name) = root_name
                        && let Some((_, package)) =
                            all_packages.iter().find(|(n, _)| n == name)
                    {
                        package_info["version"] =
                            json!(package.next_version.semver.to_string());
                    }

                    // Update dependencies within root package entry
                    if let Some(deps) = package_info.get_mut("dependencies")
                        && let Some(deps_obj) = deps.as_object_mut()
                    {
                        for (dep_name, dep_info) in deps_obj {
                            if let Some((_, package)) =
                                all_packages.iter().find(|(n, _)| n == dep_name)
                            {
                                *dep_info = json!(format!(
                                    "^{}",
                                    package.next_version.semver.to_string()
                                ));
                            }
                        }
                    }

                    // Update devDependencies within root package entry
                    if let Some(dev_deps) =
                        package_info.get_mut("devDependencies")
                        && let Some(dev_deps_obj) = dev_deps.as_object_mut()
                    {
                        for (dep_name, dep_info) in dev_deps_obj {
                            if let Some((_, package)) =
                                all_packages.iter().find(|(n, _)| n == dep_name)
                            {
                                *dep_info = json!(format!(
                                    "^{}",
                                    package.next_version.semver.to_string()
                                ));
                            }
                        }
                    }
                    continue;
                }

                // Extract package name from node_modules/ key
                if let Some(package_name) = key.strip_prefix("node_modules/")
                    && let Some((_, package)) =
                        all_packages.iter().find(|(n, _)| n == package_name)
                {
                    package_info["version"] =
                        json!(package.next_version.semver.to_string());
                }
            }
        }

        let formatted_json = serde_json::to_string_pretty(&lock_doc)?;

        Ok(Some(FileChange {
            path: lock_path.display().to_string(),
            content: formatted_json,
            update_type: crate::forge::request::FileUpdateType::Replace,
        }))
    }

    /// Update yarn.lock file for a specific package
    async fn update_yarn_lock_for_package(
        &self,
        current_package: (&str, &Package),
        other_packages: &[(String, Package)],
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        let lock_path = Path::new(&current_package.1.path).join("yarn.lock");
        let lock_path = lock_path.display().to_string();

        let content = loader.get_file_content(&lock_path).await?;

        if content.is_none() {
            return Ok(None);
        }

        info!("Updating yarn.lock at {lock_path}");
        let content = content.unwrap();
        let mut lines: Vec<String> = vec![];

        // Regex to match package entries like "package@^1.0.0:"
        let package_regex = Regex::new(r#"^"?([^@"]+)@[^"]*"?:$"#)?;
        let version_regex = Regex::new(r#"^(\s+version\s+)"(.*)""#)?;

        let mut current_yarn_package: Option<String> = None;

        for line in content.lines() {
            // Check if this line starts a new package entry
            if let Some(caps) = package_regex.captures(line) {
                current_yarn_package = Some(caps[1].to_string());
                lines.push(line.to_string());
                continue;
            }

            // Check if this is a version line and we're in a relevant package
            if let (Some(pkg_name), Some(caps)) =
                (current_yarn_package.as_ref(), version_regex.captures(line))
            {
                // Check if it matches the current package
                if current_package.0 == pkg_name {
                    let new_line = format!(
                        "{}\"{}\"",
                        &caps[1], current_package.1.next_version.semver
                    );
                    lines.push(new_line);
                    continue;
                }
                // Check if it matches one of the other packages
                else if let Some((_, package)) =
                    other_packages.iter().find(|(n, _)| n == pkg_name)
                {
                    let new_line = format!(
                        "{}\"{}\"",
                        &caps[1], package.next_version.semver
                    );
                    lines.push(new_line);
                    continue;
                }
            }

            // Reset current package when we hit an empty line or start of
            // new entry
            if line.trim().is_empty()
                || (!line.starts_with(' ')
                    && !line.starts_with('\t')
                    && line.contains(':'))
            {
                current_yarn_package = None;
            }

            lines.push(line.to_string());
        }

        let updated_content = lines.join("\n");
        Ok(Some(FileChange {
            path: lock_path,
            content: updated_content,
            update_type: FileUpdateType::Replace,
        }))
    }

    /// Update yarn.lock file at root path
    async fn update_yarn_lock_for_root(
        &self,
        root_path: &Path,
        all_packages: &[(String, Package)],
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        let lock_path = root_path.join("yarn.lock");
        let lock_path = lock_path.display().to_string();
        let content = loader.get_file_content(&lock_path).await?;

        if content.is_none() {
            return Ok(None);
        }

        info!("Updating yarn.lock at {lock_path}");

        let content = content.unwrap();
        let mut lines: Vec<String> = vec![];

        // Regex to match package entries like "package@^1.0.0:"
        let package_regex = Regex::new(r#"^"?([^@"]+)@[^"]*"?:$"#)?;
        let version_regex = Regex::new(r#"^(\s+version\s+)"(.*)""#)?;

        let mut current_yarn_package: Option<String> = None;

        for line in content.lines() {
            // Check if this line starts a new package entry
            if let Some(caps) = package_regex.captures(line) {
                current_yarn_package = Some(caps[1].to_string());
                lines.push(line.to_string());
                continue;
            }

            // Check if this is a version line and we're in a relevant package
            if let (Some(pkg_name), Some(caps)) =
                (current_yarn_package.as_ref(), version_regex.captures(line))
                && let Some((_, package)) =
                    all_packages.iter().find(|(n, _)| n == pkg_name)
            {
                let new_line =
                    format!("{}\"{}\"", &caps[1], package.next_version.semver);
                lines.push(new_line);
                continue;
            }

            // Reset current package when we hit an empty line or start of new entry
            if line.trim().is_empty()
                || (!line.starts_with(' ')
                    && !line.starts_with('\t')
                    && line.contains(':'))
            {
                current_yarn_package = None;
            }

            lines.push(line.to_string());
        }

        let updated_content = lines.join("\n");

        Ok(Some(FileChange {
            path: lock_path,
            content: updated_content,
            update_type: FileUpdateType::Replace,
        }))
    }

    /// Update lock files for a specific package
    async fn update_package_lock_files(
        &self,
        current_package: (&str, &Package),
        other_packages: &[(String, Package)],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut changes: Vec<FileChange> = vec![];

        if let Some(change) = self
            .update_package_lock_json_for_package(
                current_package,
                other_packages,
                loader,
            )
            .await?
        {
            changes.push(change);
        }

        if let Some(change) = self
            .update_yarn_lock_for_package(
                current_package,
                other_packages,
                loader,
            )
            .await?
        {
            changes.push(change);
        }

        if changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(changes))
    }

    /// Update lock files at root path
    async fn update_root_lock_files(
        &self,
        root_path: &Path,
        all_packages: &[(String, Package)],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut changes: Vec<FileChange> = vec![];

        if let Some(change) = self
            .update_package_lock_json_for_root(root_path, all_packages, loader)
            .await?
        {
            changes.push(change);
        }

        if let Some(change) = self
            .update_yarn_lock_for_root(root_path, all_packages, loader)
            .await?
        {
            changes.push(change);
        }

        if changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(changes))
    }
}

#[async_trait]
impl PackageUpdater for NodeUpdater {
    async fn update(
        &self,
        packages: Vec<Package>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let node_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Node))
            .collect::<Vec<Package>>();

        info!("Found {} node packages", node_packages.len());

        let mut file_changes: Vec<FileChange> = vec![];

        let packages_with_names =
            self.get_packages_with_names(node_packages, loader).await;

        for (package_name, package) in packages_with_names.iter() {
            let pkg_json = Path::new(&package.path).join("package.json");
            let pkg_doc = self.load_doc(pkg_json.clone(), loader).await?;
            if pkg_doc.is_none() {
                continue;
            }
            let mut pkg_doc = pkg_doc.unwrap();
            pkg_doc["version"] = json!(package.next_version.semver.to_string());

            let other_pkgs = packages_with_names
                .iter()
                .filter(|(n, _)| n != package_name)
                .cloned()
                .collect::<Vec<(String, Package)>>();

            self.update_deps(&mut pkg_doc, "dependencies", &other_pkgs)?;
            self.update_deps(&mut pkg_doc, "dev_dependencies", &other_pkgs)?;

            let formatted_json = serde_json::to_string_pretty(&pkg_doc)?;

            file_changes.push(FileChange {
                path: pkg_json.display().to_string(),
                content: formatted_json,
                update_type: FileUpdateType::Replace,
            });

            // Update lock files in this package directory
            if let Some(changes) = self
                .update_package_lock_files(
                    (package_name, package),
                    &other_pkgs,
                    loader,
                )
                .await?
            {
                file_changes.extend(changes);
            }
        }

        // Update lock files at root path
        if let Some(changes) = self
            .update_root_lock_files(
                Path::new("."),
                &packages_with_names,
                loader,
            )
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
    use semver::Version as SemVer;

    fn create_test_package(
        name: &str,
        path: &str,
        next_version: &str,
    ) -> Package {
        Package {
            name: name.to_string(),
            path: path.to_string(),
            framework: Framework::Node,
            next_version: Tag {
                sha: "test-sha".to_string(),
                name: format!("v{}", next_version),
                semver: SemVer::parse(next_version).unwrap(),
            },
        }
    }

    #[tokio::test]
    async fn test_load_doc() {
        let updater = NodeUpdater::new();
        let package_json = r#"{
  "name": "test-package",
  "version": "1.0.0"
}"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-package/package.json"))
            .times(1)
            .returning(move |_| Ok(Some(package_json.to_string())));

        let result = updater
            .load_doc("test-package/package.json", &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let doc = result.unwrap();
        assert_eq!(doc["name"], "test-package");
        assert_eq!(doc["version"], "1.0.0");
    }

    #[tokio::test]
    async fn test_load_doc_file_not_found() {
        let updater = NodeUpdater::new();

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-package/package.json"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater
            .load_doc("test-package/package.json", &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_deps() {
        let updater = NodeUpdater::new();
        let mut doc = json!({
            "name": "test-package",
            "version": "1.0.0",
            "dependencies": {
                "other-package": "^1.0.0",
                "external-package": "^2.0.0"
            }
        });

        let other_packages = vec![(
            "other-package".to_string(),
            create_test_package("other-package", "packages/other", "2.0.0"),
        )];

        updater
            .update_deps(&mut doc, "dependencies", &other_packages)
            .unwrap();

        assert_eq!(doc["dependencies"]["other-package"], "2.0.0");
        assert_eq!(doc["dependencies"]["external-package"], "^2.0.0");
    }

    #[tokio::test]
    async fn test_update_deps_skips_workspace() {
        let updater = NodeUpdater::new();
        let mut doc = json!({
            "name": "test-package",
            "version": "1.0.0",
            "dependencies": {
                "other-package": "workspace:*",
                "another-package": "^1.0.0"
            }
        });

        let other_packages = vec![
            (
                "other-package".to_string(),
                create_test_package("other-package", "packages/other", "2.0.0"),
            ),
            (
                "another-package".to_string(),
                create_test_package(
                    "another-package",
                    "packages/another",
                    "3.0.0",
                ),
            ),
        ];

        updater
            .update_deps(&mut doc, "dependencies", &other_packages)
            .unwrap();

        // workspace dependency should not be updated
        assert_eq!(doc["dependencies"]["other-package"], "workspace:*");
        // regular dependency should be updated
        assert_eq!(doc["dependencies"]["another-package"], "3.0.0");
    }

    #[tokio::test]
    async fn test_update_package_json() {
        let updater = NodeUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let package_json = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "other-package": "^1.0.0"
  }
}"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/package.json"))
            .times(2) // Called twice: once in get_packages_with_names, once in update
            .returning(move |_| Ok(Some(package_json.to_string())));

        // Mock for other files (none exist)
        mock_loader
            .expect_get_file_content()
            .returning(|_path| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/package.json");
        assert!(changes[0].content.contains("\"version\": \"2.0.0\""));
    }

    #[tokio::test]
    async fn test_update_package_lock_json_for_package() {
        let updater = NodeUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let lock_json = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "packages": {
    "": {
      "name": "test-package",
      "version": "1.0.0"
    },
    "node_modules/test-package": {
      "version": "1.0.0"
    }
  }
}"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/package-lock.json"))
            .times(1)
            .returning(move |_| Ok(Some(lock_json.to_string())));

        let result = updater
            .update_package_lock_json_for_package(
                ("test-package", &package),
                &[],
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.path, "packages/test/package-lock.json");
        assert!(change.content.contains("\"version\": \"2.0.0\""));
    }

    #[tokio::test]
    async fn test_update_package_lock_json_updates_dependencies() {
        let updater = NodeUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let lock_json = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "packages": {
    "": {
      "name": "test-package",
      "version": "1.0.0",
      "dependencies": {
        "other-package": "^1.0.0"
      }
    },
    "node_modules/other-package": {
      "version": "1.0.0"
    }
  }
}"#;

        let other_packages = vec![(
            "other-package".to_string(),
            create_test_package("other-package", "packages/other", "3.0.0"),
        )];

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/package-lock.json"))
            .times(1)
            .returning(move |_| Ok(Some(lock_json.to_string())));

        let result = updater
            .update_package_lock_json_for_package(
                ("test-package", &package),
                &other_packages,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        // Should update the root package version
        assert!(change.content.contains("\"version\": \"2.0.0\""));
        // Should update dependency reference
        assert!(change.content.contains("\"other-package\": \"^3.0.0\""));
        // Should update node_modules version
        assert!(change.content.contains("\"version\": \"3.0.0\""));
    }

    #[tokio::test]
    async fn test_update_package_lock_json_file_not_found() {
        let updater = NodeUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/package-lock.json"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater
            .update_package_lock_json_for_package(
                ("test-package", &package),
                &[],
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_yarn_lock_for_package() {
        let updater = NodeUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let yarn_lock = r#"# THIS IS AN AUTOGENERATED FILE.
# yarn lockfile v1

"test-package@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/test-package/-/test-package-1.0.0.tgz"

"other-package@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/other-package/-/other-package-1.0.0.tgz"
"#;

        let other_packages = vec![(
            "other-package".to_string(),
            create_test_package("other-package", "packages/other", "3.0.0"),
        )];

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/yarn.lock"))
            .times(1)
            .returning(move |_| Ok(Some(yarn_lock.to_string())));

        let result = updater
            .update_yarn_lock_for_package(
                ("test-package", &package),
                &other_packages,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.path, "packages/test/yarn.lock");
        assert!(change.content.contains("version \"2.0.0\""));
        assert!(change.content.contains("version \"3.0.0\""));
    }

    #[tokio::test]
    async fn test_update_yarn_lock_file_not_found() {
        let updater = NodeUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/yarn.lock"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater
            .update_yarn_lock_for_package(
                ("test-package", &package),
                &[],
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_root_lock_files() {
        let updater = NodeUpdater::new();

        let lock_json = r#"{
  "name": "root",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "packages": {
    "": {
      "name": "root",
      "version": "1.0.0"
    },
    "node_modules/test-package": {
      "version": "1.0.0"
    }
  }
}"#;

        let yarn_lock = r#"# yarn lockfile v1

"test-package@^1.0.0":
  version "1.0.0"
"#;

        let packages = vec![(
            "test-package".to_string(),
            create_test_package("test-package", "packages/test", "2.0.0"),
        )];

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./package-lock.json"))
            .times(1)
            .returning({
                let content = lock_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("./yarn.lock"))
            .times(1)
            .returning({
                let content = yarn_lock.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let result = updater
            .update_root_lock_files(Path::new("."), &packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);

        // Check package-lock.json was updated
        assert!(changes.iter().any(|c| c.path == "./package-lock.json"
            && c.content.contains("\"version\": \"2.0.0\"")));

        // Check yarn.lock was updated
        assert!(changes.iter().any(|c| c.path == "./yarn.lock"
            && c.content.contains("version \"2.0.0\"")));
    }

    #[tokio::test]
    async fn test_update_multiple_packages() {
        let updater = NodeUpdater::new();

        let package1_json = r#"{
  "name": "package-one",
  "version": "1.0.0",
  "dependencies": {
    "package-two": "^1.0.0"
  }
}"#;

        let package2_json = r#"{
  "name": "package-two",
  "version": "1.0.0"
}"#;

        let packages = vec![
            create_test_package("package-one", "packages/one", "2.0.0"),
            create_test_package("package-two", "packages/two", "3.0.0"),
        ];

        let mut mock_loader = MockFileLoader::new();

        // Mock for package-one's package.json
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/one/package.json"))
            .times(2) // Called twice: once in get_packages_with_names, once in update
            .returning({
                let content = package1_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Mock for package-two's package.json
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/two/package.json"))
            .times(2)
            .returning({
                let content = package2_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Mock for other files (none exist)
        mock_loader
            .expect_get_file_content()
            .returning(|_path| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);

        // Check package-one was updated
        let pkg1_change = changes
            .iter()
            .find(|c| c.path == "packages/one/package.json")
            .unwrap();
        assert!(pkg1_change.content.contains("\"version\": \"2.0.0\""));
        assert!(pkg1_change.content.contains("\"package-two\": \"3.0.0\""));

        // Check package-two was updated
        let pkg2_change = changes
            .iter()
            .find(|c| c.path == "packages/two/package.json")
            .unwrap();
        assert!(pkg2_change.content.contains("\"version\": \"3.0.0\""));
    }

    #[tokio::test]
    async fn test_update_filters_node_packages() {
        let updater = NodeUpdater::new();

        let packages = vec![
            create_test_package("node-package", "packages/node", "2.0.0"),
            Package {
                name: "java-package".to_string(),
                path: "packages/java".to_string(),
                framework: Framework::Java,
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
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        // Should return None when no package.json files are found
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_packages_with_names() {
        let updater = NodeUpdater::new();

        let package1_json = r#"{
  "name": "custom-name",
  "version": "1.0.0"
}"#;

        let packages = vec![
            create_test_package("package-one", "packages/one", "2.0.0"),
            create_test_package("package-two", "packages/two", "3.0.0"),
        ];

        let mut mock_loader = MockFileLoader::new();

        // First package has a custom name in package.json
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/one/package.json"))
            .times(1)
            .returning({
                let content = package1_json.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Second package doesn't have package.json
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/two/package.json"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater
            .get_packages_with_names(packages, &mock_loader)
            .await;

        assert_eq!(result.len(), 2);
        // First package should use name from package.json
        assert_eq!(result[0].0, "custom-name");
        // Second package should use package name as fallback
        assert_eq!(result[1].0, "package-two");
    }

    #[tokio::test]
    async fn test_update_package_lock_json_with_dev_dependencies() {
        let updater = NodeUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let lock_json = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "packages": {
    "": {
      "name": "test-package",
      "version": "1.0.0",
      "devDependencies": {
        "other-package": "^1.0.0"
      }
    },
    "node_modules/other-package": {
      "version": "1.0.0"
    }
  }
}"#;

        let other_packages = vec![(
            "other-package".to_string(),
            create_test_package("other-package", "packages/other", "3.0.0"),
        )];

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/package-lock.json"))
            .times(1)
            .returning(move |_| Ok(Some(lock_json.to_string())));

        let result = updater
            .update_package_lock_json_for_package(
                ("test-package", &package),
                &other_packages,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        // Should update devDependency reference
        assert!(change.content.contains("\"other-package\": \"^3.0.0\""));
    }
}
