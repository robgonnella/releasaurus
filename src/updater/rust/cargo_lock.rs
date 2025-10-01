use color_eyre::eyre::ContextCompat;
use std::path::Path;
use toml_edit::{DocumentMut, value};

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
    updater::framework::Package,
};

pub struct CargoLock {}

impl CargoLock {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn process_workspace_lockfile(
        &self,
        root_path: &Path,
        packages: &[(String, Package)],
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        let lock_path = root_path.join("Cargo.lock");
        let lock_doc = self.load_doc(lock_path.as_path(), loader).await?;
        if lock_doc.is_none() {
            return Ok(None);
        }
        let mut lock_doc = lock_doc.unwrap();
        let lock_packages = lock_doc["package"]
            .as_array_of_tables_mut()
            .wrap_err("Cargo.lock doesn't seem to have any packages")?;

        let mut updated = 0;

        for lock_pkg in lock_packages.iter_mut() {
            if updated == packages.len() {
                break;
            }

            for (package_name, package) in packages.iter() {
                if let Some(name) = lock_pkg.get("name")
                    && let Some(name_str) = name.as_str()
                    && package_name == name_str
                    && let Some(version) = lock_pkg.get_mut("version")
                {
                    *version = value(package.next_version.semver.to_string());
                    updated += 1;
                }
            }
        }

        if updated > 0 {
            return Ok(Some(FileChange {
                path: lock_path.display().to_string(),
                content: lock_doc.to_string(),
                update_type: FileUpdateType::Replace,
            }));
        }

        Ok(None)
    }

    pub async fn process_packages(
        &self,
        packages: &[(String, Package)],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for (_package_name, package) in packages.iter() {
            let mut updated = false;
            let doc_path = Path::new(&package.path).join("Cargo.lock");
            let lock_doc = self.load_doc(doc_path.as_path(), loader).await?;

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
                    path: doc_path.display().to_string(),
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

    async fn load_doc<P: AsRef<Path>>(
        &self,
        file_path: P,
        loader: &dyn FileLoader,
    ) -> Result<Option<DocumentMut>> {
        let file_path = file_path.as_ref().display().to_string();
        let content = loader.get_file_content(&file_path).await?;
        if content.is_none() {
            return Ok(None);
        }
        let content = content.unwrap();
        let doc = content.parse::<DocumentMut>()?;
        Ok(Some(doc))
    }
}
