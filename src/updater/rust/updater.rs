//! Cargo updater for handling rust projects
use color_eyre::eyre::Result;
use log::*;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;
use toml_edit::{DocumentMut, value};

use crate::updater::framework::{Framework, Package};
use crate::updater::traits::PackageUpdater;

pub struct CargoUpdater {}

impl CargoUpdater {
    pub fn new() -> Self {
        Self {}
    }

    pub fn load_doc(&self, package_path: &str) -> Result<DocumentMut> {
        let file_path = Path::new(&package_path).join("Cargo.toml");
        let mut file = OpenOptions::new().read(true).open(file_path)?;
        let mut content = String::from("");
        file.read_to_string(&mut content)?;
        let doc = content.parse::<DocumentMut>()?;
        Ok(doc)
    }

    fn write_doc(
        &self,
        doc: &mut DocumentMut,
        package_path: &str,
    ) -> Result<()> {
        let file_path = Path::new(&package_path).join("Cargo.toml");
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(file_path)?;
        file.write_all(doc.to_string().as_bytes())?;
        Ok(())
    }

    fn get_package_name(&self, doc: &DocumentMut, package: &Package) -> String {
        let mut package_name = package.name.clone();

        if doc
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("name"))
            .is_some()
        {
            package_name = doc["package"]["name"].to_string()
        }

        package_name
    }

    fn process_doc_dependencies(
        &self,
        doc: &mut DocumentMut,
        package_name: &str,
        next_version: &str,
        kind: &str,
    ) -> Result<bool> {
        let mut updated = false;

        let dep_exists = doc
            .get(kind)
            .and_then(|deps| deps.as_table())
            .and_then(|t| t.get(package_name))
            .is_some();

        let is_version_object = doc
            .get(kind)
            .and_then(|deps| deps.as_table())
            .and_then(|t| t.get(package_name))
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("version"))
            .is_some();

        if dep_exists {
            if is_version_object {
                doc[kind][&package_name]["version"] = value(next_version);
            } else {
                doc[kind][&package_name] = value(next_version);
            }
            updated = true;
        }

        Ok(updated)
    }
}

impl PackageUpdater for CargoUpdater {
    fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()> {
        info!(
            "Found {} rust packages in {}",
            packages.len(),
            root_path.display(),
        );

        let rust_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Rust))
            .collect::<Vec<Package>>();

        for package in rust_packages.iter() {
            let mut doc = self.load_doc(&package.path)?;

            if doc.get("workspace").is_some() {
                debug!("skipping cargo workspace file");
                continue;
            }

            if doc.get("package").is_none() {
                warn!(
                    "found Cargo.toml manifest but [package] section is missing: skipping"
                );
                continue;
            }

            let package_name = self.get_package_name(&doc, package);
            let next_version = package.next_version.semver.to_string();

            info!("setting version for {package_name} to {next_version}");

            doc["package"]["version"] = value(&next_version);

            self.write_doc(&mut doc, &package.path)?;

            // iterate through packages again to update any other packages that
            // depend on this package
            for c in rust_packages.iter() {
                let mut dep_doc = self.load_doc(&c.path)?;

                let dep_name = self.get_package_name(&dep_doc, c);

                if dep_name == package_name {
                    info!("skipping dep check on self: {dep_name}");
                    continue;
                }

                let dep_updated = self.process_doc_dependencies(
                    &mut dep_doc,
                    &package_name,
                    &next_version,
                    "dependencies",
                )?;

                let dev_updated = self.process_doc_dependencies(
                    &mut dep_doc,
                    &package_name,
                    &next_version,
                    "dev-dependencies",
                )?;

                let build_updated = self.process_doc_dependencies(
                    &mut dep_doc,
                    &package_name,
                    &next_version,
                    "build-dependencies",
                )?;

                if dep_updated || dev_updated || build_updated {
                    self.write_doc(&mut dep_doc, &c.path)?;
                }
            }
        }

        Ok(())
    }
}
