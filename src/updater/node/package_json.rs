use log::*;
use serde_json::{Value, json};

use crate::{
    forge::request::{FileChange, FileUpdateType},
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
    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename != "package.json" {
                continue;
            }

            let mut doc = self.load_doc(&manifest.content)?;
            doc["version"] = json!(package.next_version.semver.to_string());

            let other_pkgs = workspace_packages
                .iter()
                .filter(|p| p.package_name != package.package_name)
                .cloned()
                .collect::<Vec<UpdaterPackage>>();

            self.update_deps(&mut doc, "dependencies", &other_pkgs)?;
            self.update_deps(&mut doc, "devDependencies", &other_pkgs)?;

            let formatted_json = serde_json::to_string_pretty(&doc)?;

            file_changes.push(FileChange {
                path: manifest.file_path.clone(),
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
        other_packages: &[UpdaterPackage],
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

                if let Some(package) =
                    other_packages.iter().find(|p| p.package_name == dep_name)
                {
                    deps[&dep_name] =
                        json!(format!("^{}", package.next_version.semver));
                }
            }
        }

        Ok(())
    }

    fn load_doc(&self, content: &str) -> Result<Value> {
        let doc = serde_json::from_str(content)?;
        Ok(doc)
    }
}
