use serde_json::{Value, json};

use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::{ManifestFile, UpdaterPackage},
};

/// Handles package-lock.json file parsing and version updates for Node.js packages.
pub struct PackageLock {}

impl PackageLock {
    /// Create package-lock.json handler for version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version fields in package-lock.json files for all Node packages.
    pub async fn process_package(
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
                && let Some(change) = self
                    .update_lock_file(manifest, package, workspace_packages)
                    .await?
            {
                file_changes.push(change);
            } else if let Some(change) =
                self.update_lock_file(manifest, package, &[]).await?
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
