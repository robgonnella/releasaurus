use log::*;
use regex::Regex;
use std::path::Path;

use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::UpdaterPackage,
};

/// Handles .gemspec file parsing and version updates for Ruby packages.
pub struct Gemspec {}

impl Gemspec {
    /// Create Gemspec handler for .gemspec version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version string in gemspec file using regex pattern matching.
    fn update_version(&self, content: &str, new_version: &str) -> String {
        // Match patterns like:
        // spec.version = "1.0.0"
        // spec.version = '1.0.0'
        // s.version = "1.0.0"
        let re =
            Regex::new(r#"((?:spec|s)\.version\s*=\s*)(["'])([^"']+)(["'])"#)
                .unwrap();

        re.replace_all(content, |caps: &regex::Captures| {
            format!("{}{}{}{}", &caps[1], &caps[2], new_version, &caps[4])
        })
        .to_string()
    }

    /// Process gemspec files for all Ruby packages.
    pub async fn process_packages(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        for manifest in package.manifest_files.iter() {
            let file_path = Path::new(&manifest.file_basename);
            let file_ext = file_path.extension();

            if file_ext.is_none() {
                continue;
            }

            let file_ext = file_ext.unwrap();

            if file_ext.display().to_string() != "gemspec" {
                continue;
            }

            info!("processing gemspec file: {}", manifest.file_basename);

            let updated_content = self.update_version(
                &manifest.content,
                &package.next_version.semver.to_string(),
            );

            if updated_content != manifest.content {
                file_changes.push(FileChange {
                    path: manifest.file_path.clone(),
                    content: updated_content,
                    update_type: FileUpdateType::Replace,
                });
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}
