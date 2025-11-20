// use log::*;
use regex::Regex;

use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::{ManifestFile, UpdaterPackage},
};

/// Handles version.rb file parsing and version updates for Ruby packages.
pub struct VersionRb {}

impl VersionRb {
    /// Create VersionRb handler for version.rb version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Process version.rb files for all Ruby packages.
    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename == "version.rb"
                && let Some(change) = self.update_version(manifest, package)
            {
                file_changes.push(change);
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    /// Update version in a version.rb file.
    fn update_version(
        &self,
        manifest: &ManifestFile,
        package: &UpdaterPackage,
    ) -> Option<FileChange> {
        // Match patterns like:
        // VERSION = "1.0.0"
        // VERSION = '1.0.0'
        let re = Regex::new(r#"(VERSION\s*=\s*)(["'])([^"']+)(["'])"#).unwrap();

        if !re.is_match(&manifest.content) {
            return None;
        }

        let updated_content = re
            .replace_all(&manifest.content, |caps: &regex::Captures| {
                format!(
                    "{}{}{}{}",
                    &caps[1], &caps[2], package.next_version, &caps[4]
                )
            })
            .to_string();

        Some(FileChange {
            path: manifest.file_path.clone(),
            content: updated_content,
            update_type: FileUpdateType::Replace,
        })
    }
}
