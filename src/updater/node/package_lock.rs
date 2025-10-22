use log::*;
use serde_json::{Value, json};
use std::collections::HashSet;

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
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
    pub async fn process_packages(
        &self,
        packages: &[(String, UpdaterPackage)],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];
        let mut processed_paths = HashSet::new();

        // First, handle workspace-level package-lock.json files
        let mut workspace_roots: Vec<&str> = packages
            .iter()
            .map(|(_, p)| p.workspace_root.as_str())
            .collect();
        workspace_roots.sort_unstable();
        workspace_roots.dedup();

        for workspace_root in workspace_roots {
            // Get a package from this workspace to use its helper method
            let workspace_package = packages
                .iter()
                .find(|(_, p)| p.workspace_root == workspace_root)
                .map(|(_, p)| p);

            if workspace_package.is_none() {
                continue;
            }

            let workspace_package = workspace_package.unwrap();
            let workspace_lock_path =
                workspace_package.get_workspace_file_path("package-lock.json");

            let workspace_packages: Vec<(String, UpdaterPackage)> = packages
                .iter()
                .filter(|(_, p)| p.workspace_root == workspace_root)
                .cloned()
                .collect();

            if let Some(change) = self
                .update_lock_file(
                    &workspace_lock_path,
                    &workspace_packages,
                    loader,
                )
                .await?
            {
                processed_paths.insert(change.path.clone());
                file_changes.push(change);
            }
        }

        // Then handle package-level package-lock.json files
        for (package_name, package) in packages.iter() {
            let path_str = package.get_file_path("package-lock.json");

            // Skip if this path was already processed as a workspace lock file
            if processed_paths.contains(&path_str) {
                continue;
            }

            if let Some(change) = self
                .update_lock_file(
                    &path_str,
                    &[(package_name.clone(), package.clone())],
                    loader,
                )
                .await?
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
    async fn update_lock_file(
        &self,
        lock_path: &str,
        all_packages: &[(String, UpdaterPackage)],
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        let lock_doc = self.load_doc(lock_path, loader).await?;

        if lock_doc.is_none() {
            return Ok(None);
        }

        info!("Updating package-lock.json at {}", lock_path);
        let mut lock_doc = lock_doc.unwrap();

        // Get root package name for later use
        let root_name = lock_doc
            .get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        // Update root level version if this lock file corresponds to one of our packages
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
                    // Root package entry - update version if this corresponds to one of our packages
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
                            // Skip workspace: and repo: protocol dependencies
                            if let Some(version_str) = dep_info.as_str()
                                && (version_str.starts_with("workspace:")
                                    || version_str.starts_with("repo:"))
                            {
                                continue;
                            }

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
                            // Skip workspace: and repo: protocol dependencies
                            if let Some(version_str) = dep_info.as_str()
                                && (version_str.starts_with("workspace:")
                                    || version_str.starts_with("repo:"))
                            {
                                continue;
                            }

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
            path: lock_path.to_string(),
            content: formatted_json,
            update_type: FileUpdateType::Replace,
        }))
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
