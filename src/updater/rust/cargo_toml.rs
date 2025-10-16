use log::*;
use toml_edit::{DocumentMut, value};

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
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
    pub async fn process_packages(
        &self,
        packages: &[(String, UpdaterPackage)],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for (package_name, package) in packages.iter() {
            let manifest_path = package.get_file_path("Cargo.toml");

            let doc = self.load_doc(&manifest_path, loader).await?;

            if doc.is_none() {
                continue;
            }

            let mut doc = doc.unwrap();

            if doc.get("workspace").is_some() {
                debug!("skipping cargo workspace file");
                continue;
            }

            let next_version = package.next_version.semver.to_string();

            info!("setting version for {package_name} to {next_version}");

            doc["package"]["version"] = value(&next_version);

            let other_pkgs = packages
                .iter()
                .filter(|(n, _)| n != package_name)
                .cloned()
                .collect::<Vec<(String, UpdaterPackage)>>();

            // loop other packages to check if they current manifest deps
            for (dep_name, dep) in other_pkgs.iter() {
                let dep_next_version = dep.next_version.semver.to_string();

                self.process_dependencies(
                    &mut doc,
                    dep_name,
                    &dep_next_version,
                    "dependencies",
                );

                self.process_dependencies(
                    &mut doc,
                    dep_name,
                    &dep_next_version,
                    "dev-dependencies",
                );

                self.process_dependencies(
                    &mut doc,
                    dep_name,
                    &dep_next_version,
                    "build-dependencies",
                );
            }

            file_changes.push(FileChange {
                path: manifest_path,
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
