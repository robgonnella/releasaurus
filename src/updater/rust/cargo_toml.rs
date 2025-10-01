use log::*;
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

pub struct CargoToml {}

impl CargoToml {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn is_workspace(
        &self,
        root_path: &Path,
        loader: &dyn FileLoader,
    ) -> Result<bool> {
        let file_path = root_path.join("Cargo.toml");
        let doc = self.load_doc(file_path, loader).await?;
        if doc.is_none() {
            return Ok(false);
        }
        let doc = doc.unwrap();
        Ok(doc.get("workspace").is_some())
    }

    pub async fn get_packages_with_names(
        &self,
        packages: Vec<Package>,
        loader: &dyn FileLoader,
    ) -> Vec<(String, Package)> {
        let results = packages.into_iter().map(|p| async {
            let manifest_path = Path::new(&p.path).join("Cargo.toml");
            let doc = self.load_doc(manifest_path, loader).await;
            if let Ok(doc) = doc
                && let Some(doc) = doc
            {
                let pkg_name = self.get_package_name(&doc, &p);
                return (pkg_name, p);
            }
            (p.name.clone(), p)
        });

        futures::future::join_all(results).await
    }

    pub async fn process_packages(
        &self,
        packages: &[(String, Package)],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for (package_name, package) in packages.iter() {
            let manifest_path = Path::new(&package.path).join("Cargo.toml");

            let doc = self.load_doc(manifest_path.as_path(), loader).await?;

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
                .collect::<Vec<(String, Package)>>();

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
                path: manifest_path.display().to_string(),
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

    fn get_package_name(&self, doc: &DocumentMut, package: &Package) -> String {
        doc.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
            .unwrap_or(package.name.clone())
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
