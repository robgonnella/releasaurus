use async_trait::async_trait;
use log::*;
use serde_json::{Value, json};
use std::path::Path;

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
    updater::{
        framework::{Framework, Package},
        traits::PackageUpdater,
    },
};

/// PHP package updater for Composer projects.
pub struct PhpUpdater {}

impl PhpUpdater {
    pub fn new() -> Self {
        Self {}
    }

    /// Load a composer.json file as a JSON Value
    async fn load_doc<P: AsRef<Path>>(
        &self,
        file_path: P,
        loader: &dyn FileLoader,
    ) -> Result<Option<Value>> {
        let file_path = file_path.as_ref().display().to_string();
        let content = loader.get_file_content(&file_path).await?;
        if content.is_none() {
            return Ok(None);
        }
        let content = content.unwrap();
        let doc: Value = serde_json::from_str(&content)?;
        Ok(Some(doc))
    }

    /// Process packages and update their composer.json files
    async fn process_packages(
        &self,
        packages: &[Package],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];
        for package in packages {
            let file_path = Path::new(&package.path).join("composer.json");

            let doc = self.load_doc(&file_path, loader).await?;

            if doc.is_none() {
                continue;
            }

            info!("found composer.json for package: {}", package.path);
            let mut doc = doc.unwrap();

            // Update the version field
            if let Some(obj) = doc.as_object_mut() {
                info!(
                    "updating {} version to {}",
                    file_path.display(),
                    package.next_version.semver
                );

                obj.insert(
                    "version".to_string(),
                    json!(package.next_version.semver.to_string()),
                );

                let formatted = serde_json::to_string_pretty(&doc)?;

                file_changes.push(FileChange {
                    path: file_path.display().to_string(),
                    content: formatted,
                    update_type: FileUpdateType::Replace,
                });
            } else {
                warn!(
                    "composer.json is not a valid JSON object: {}",
                    file_path.display()
                );
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}

#[async_trait]
impl PackageUpdater for PhpUpdater {
    async fn update(
        &self,
        packages: Vec<Package>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let php_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Php))
            .collect::<Vec<Package>>();

        info!("Found {} PHP packages", php_packages.len(),);

        self.process_packages(&php_packages, loader).await
    }
}
