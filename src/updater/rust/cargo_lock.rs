use std::collections::HashSet;
use toml_edit::{DocumentMut, value};

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
    updater::framework::UpdaterPackage,
};

/// Handles Cargo.lock file parsing and version synchronization for Rust
/// workspace dependencies.
pub struct CargoLock {}

impl CargoLock {
    /// Create Cargo.lock handler for lockfile version updates.
    pub fn new() -> Self {
        Self {}
    }

    pub async fn process_packages(
        &self,
        packages: &[(String, UpdaterPackage)],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];
        let mut processed_paths = HashSet::new();

        // First, handle workspace-level Cargo.lock files
        // Collect unique workspace roots
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
                workspace_package.get_workspace_file_path("Cargo.lock");

            let lock_doc = self.load_doc(&workspace_lock_path, loader).await?;

            if let Some(mut lock_doc) = lock_doc {
                let mut updated = false;

                if let Some(doc_packages) =
                    lock_doc["package"].as_array_of_tables_mut()
                {
                    // Update all packages in this workspace
                    for (dep_name, dep) in packages.iter() {
                        if dep.workspace_root == workspace_root
                            && let Some(found) =
                                doc_packages.iter_mut().find(|p| {
                                    let doc_package_name = p
                                        .get("name")
                                        .and_then(|item| item.as_str())
                                        .unwrap_or("");
                                    doc_package_name == dep_name
                                })
                        {
                            found["version"] =
                                value(dep.next_version.semver.to_string());
                            updated = true;
                        }
                    }
                }

                if updated {
                    processed_paths.insert(workspace_lock_path.clone());
                    file_changes.push(FileChange {
                        path: workspace_lock_path,
                        content: lock_doc.to_string(),
                        update_type: FileUpdateType::Replace,
                    });
                }
            }
        }

        // Then handle package-level Cargo.lock files
        for (_package_name, package) in packages.iter() {
            let path_str = package.get_file_path("Cargo.lock");

            // Skip if we already processed this path as a workspace lock
            if processed_paths.contains(&path_str) {
                continue;
            }

            let mut updated = false;
            let lock_doc = self.load_doc(&path_str, loader).await?;

            if lock_doc.is_none() {
                continue;
            }

            let mut lock_doc = lock_doc.unwrap();

            let doc_packages = lock_doc["package"].as_array_of_tables_mut();

            if doc_packages.is_none() {
                continue;
            }

            let doc_packages = doc_packages.unwrap();

            for (dep_name, dep) in packages.iter() {
                if let Some(found) = doc_packages.iter_mut().find(|p| {
                    let doc_package_name = p
                        .get("name")
                        .and_then(|item| item.as_str())
                        .unwrap_or("");
                    doc_package_name == dep_name
                }) {
                    found["version"] =
                        value(dep.next_version.semver.to_string());
                    updated = true;
                }
            }

            if updated {
                file_changes.push(FileChange {
                    path: path_str,
                    content: lock_doc.to_string(),
                    update_type: FileUpdateType::Replace,
                });
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    async fn load_doc(
        &self,
        file_path: &str,
        loader: &dyn FileLoader,
    ) -> Result<Option<DocumentMut>> {
        let content = loader.get_file_content(file_path).await?;
        if content.is_none() {
            return Ok(None);
        }
        let content = content.unwrap();
        let doc = content.parse::<DocumentMut>()?;
        Ok(Some(doc))
    }
}
