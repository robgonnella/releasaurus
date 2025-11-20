use toml_edit::{DocumentMut, value};

use crate::{
    forge::request::{FileChange, FileUpdateType},
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

    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename != "Cargo.lock" {
                continue;
            }

            let mut lock_doc = self.load_doc(&manifest.content)?;

            if let Some(doc_packages) =
                lock_doc["package"].as_array_of_tables_mut()
            {
                let mut updated = false;
                // Update all packages in this workspace
                for pkg in workspace_packages.iter() {
                    if let Some(found) = doc_packages.iter_mut().find(|p| {
                        let doc_package_name = p
                            .get("name")
                            .and_then(|item| item.as_str())
                            .unwrap_or("");
                        doc_package_name == pkg.package_name
                    }) {
                        found["version"] =
                            value(pkg.next_version.semver.to_string());
                        updated = true;
                    }
                }

                if updated {
                    file_changes.push(FileChange {
                        path: manifest.file_path.clone(),
                        content: lock_doc.to_string(),
                        update_type: FileUpdateType::Replace,
                    });
                }
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    fn load_doc(&self, content: &str) -> Result<DocumentMut> {
        let doc = content.parse::<DocumentMut>()?;
        Ok(doc)
    }
}
