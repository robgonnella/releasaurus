use log::*;
use regex::Regex;
// use std::collections::HashSet;

use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::UpdaterPackage,
};

/// Handles yarn.lock file parsing and version updates for Node.js packages.
pub struct YarnLock {}

impl YarnLock {
    /// Create yarn.lock handler for version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version fields in yarn.lock files for all Node packages.
    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        // Regex to match package entries like "package@^1.0.0:"
        let package_regex = Regex::new(r#"^"?([^@"]+)@[^"]*"?:$"#)?;
        let version_regex = Regex::new(r#"^(\s+version\s+)"(.*)""#)?;

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename != "yarn.lock" {
                continue;
            }

            info!("processing {}", manifest.file_path);

            let mut updated = false;
            let mut lines: Vec<String> = vec![];

            let mut current_yarn_package: Option<String> = None;

            for line in manifest.content.lines() {
                // Check if this line starts a new package entry
                if let Some(caps) = package_regex.captures(line) {
                    current_yarn_package = Some(caps[1].to_string());
                    lines.push(line.to_string());
                    continue;
                }

                // Check if this is a version line and we're in a relevant package
                if let (Some(pkg_name), Some(caps)) = (
                    current_yarn_package.as_ref(),
                    version_regex.captures(line),
                ) && let Some(pkg) = workspace_packages
                    .iter()
                    .find(|p| p.package_name == *pkg_name)
                {
                    let new_line =
                        format!("{}\"{}\"", &caps[1], pkg.next_version.semver);
                    lines.push(new_line);
                    updated = true;
                    continue;
                }

                // Reset current package when we hit an empty line or start of new entry
                if line.trim().is_empty()
                    || (!line.starts_with(' ')
                        && !line.starts_with('\t')
                        && line.contains(':'))
                {
                    current_yarn_package = None;
                }

                lines.push(line.to_string());
            }

            let updated_content = lines.join("\n");

            if updated {
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
