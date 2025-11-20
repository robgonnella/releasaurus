use log::*;
use regex::Regex;

use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::{ManifestFile, UpdaterPackage},
};

/// Handles gradle.properties file parsing and version updates for Java packages.
pub struct GradleProperties {}

impl GradleProperties {
    /// Create GradleProperties handler for gradle.properties version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version fields in gradle.properties files for all Java packages.
    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename == "gradle.properties"
                && let Some(change) =
                    self.update_properties_file(manifest, package).await?
            {
                file_changes.push(change);
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    /// Update a single gradle.properties file
    async fn update_properties_file(
        &self,
        manifest: &ManifestFile,
        package: &UpdaterPackage,
    ) -> Result<Option<FileChange>> {
        info!("Updating gradle.properties: {}", manifest.file_path);

        let mut lines: Vec<String> = Vec::new();
        let mut version_updated = false;

        let new_version = package.next_version.semver.to_string();

        // Read all lines and update version property
        // Regex to capture: indentation, "version", spacing around =, and old version
        let version_regex = Regex::new(r"^(\s*version\s*=\s*)(.*)$").unwrap();

        for line in manifest.content.lines() {
            if line.trim_start().starts_with("version") && line.contains('=') {
                if let Some(caps) = version_regex.captures(line) {
                    // Preserve everything before the version value
                    lines.push(format!("{}{}", &caps[1], new_version));
                    version_updated = true;
                    info!(
                        "Updated version in gradle.properties to: {}",
                        new_version
                    );
                } else {
                    lines.push(line.to_string());
                }
            } else {
                lines.push(line.to_string());
            }
        }

        // Only write back if we actually updated something
        if version_updated {
            let updated_content = lines.join("\n");
            return Ok(Some(FileChange {
                path: manifest.file_path.clone(),
                content: updated_content,
                update_type: FileUpdateType::Replace,
            }));
        }

        Ok(None)
    }
}
