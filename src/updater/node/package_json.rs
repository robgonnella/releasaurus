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
            for (dep_name, _) in deps.clone() {
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
