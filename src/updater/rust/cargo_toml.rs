use log::*;
use toml_edit::{DocumentMut, value};

use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::UpdaterPackage,
};

/// Handles Cargo.toml file parsing and version updates for Rust packages.
pub struct CargoToml {}

impl CargoToml {
    /// Create Cargo.toml handler for version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version fields in Cargo.toml files for all Rust packages.
    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename != "Cargo.toml" {
                continue;
            }

            let mut doc = self.load_doc(&manifest.content)?;

            if doc.get("workspace").is_some() {
                debug!("skipping cargo workspace file");
                continue;
            }

            let next_version = package.next_version.semver.to_string();

            info!(
                "setting version for {} to {next_version}",
                package.package_name
            );

            doc["package"]["version"] = value(&next_version);

            let other_pkgs = workspace_packages
                .iter()
                .filter(|p| p.package_name != package.package_name)
                .cloned()
                .collect::<Vec<UpdaterPackage>>();

            // loop other packages to check if they current manifest deps
            for wkspc_pkg in other_pkgs.iter() {
                let next_version = wkspc_pkg.next_version.semver.to_string();

                self.process_dependencies(
                    &mut doc,
                    &wkspc_pkg.package_name,
                    &next_version,
                    "dependencies",
                );

                self.process_dependencies(
                    &mut doc,
                    &wkspc_pkg.package_name,
                    &next_version,
                    "dev-dependencies",
                );

                self.process_dependencies(
                    &mut doc,
                    &wkspc_pkg.package_name,
                    &next_version,
                    "build-dependencies",
                );
            }

            file_changes.push(FileChange {
                path: manifest.file_path.clone(),
                content: doc.to_string(),
                update_type: FileUpdateType::Replace,
            });
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    fn process_dependencies(
        &self,
        doc: &mut DocumentMut,
        package_name: &str,
        next_version: &str,
        kind: &str,
    ) {
        let dep_exists = doc
            .get(kind)
            .and_then(|deps| deps.as_table())
            .and_then(|t| t.get(package_name))
            .is_some();

        let is_version_object = doc
            .get(kind)
            .and_then(|deps| deps.as_table())
            .and_then(|t| t.get(package_name))
            .map(|p| {
                // Check if it's a table with version field or inline table with
                //  version field
                p.as_table()
                    .map(|t| t.contains_key("version"))
                    .unwrap_or(false)
                    || p.as_inline_table()
                        .map(|t| t.contains_key("version"))
                        .unwrap_or(false)
            })
            .unwrap_or(false);

        if dep_exists {
            if is_version_object {
                doc[kind][&package_name]["version"] = value(next_version);
            } else {
                doc[kind][&package_name] = value(next_version);
            }
        }
    }

    fn load_doc(&self, content: &str) -> Result<DocumentMut> {
        let doc = content.parse::<DocumentMut>()?;
        Ok(doc)
    }
}
