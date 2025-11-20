use log::*;
use serde_json::{Value, json};

use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::UpdaterPackage,
};

/// Handles composer.json file parsing and version updates for PHP packages.
pub struct ComposerJson {}

impl ComposerJson {
    /// Create ComposerJson handler for composer.json version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Process composer.json files for all PHP packages.
    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            let doc = self.load_doc(&manifest.content).await?;

            if doc.is_none() {
                continue;
            }

            info!("found composer.json for package: {}", manifest.file_path);

            let mut doc = doc.unwrap();

            // Update the version field
            if let Some(obj) = doc.as_object_mut() {
                info!(
                    "updating {} version to {}",
                    manifest.file_path, package.next_version.semver
                );

                obj.insert(
                    "version".to_string(),
                    json!(package.next_version.semver.to_string()),
                );

                let formatted = serde_json::to_string_pretty(&doc)?;

                file_changes.push(FileChange {
                    path: manifest.file_path.clone(),
                    content: formatted,
                    update_type: FileUpdateType::Replace,
                });
            } else {
                warn!(
                    "composer.json is not a valid JSON object: {}",
                    manifest.file_path
                );
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    /// Load and parse composer.json file from repository into serde_json Value.
    async fn load_doc(&self, content: &str) -> Result<Option<Value>> {
        let doc: Value = serde_json::from_str(content)?;
        Ok(Some(doc))
    }
}
