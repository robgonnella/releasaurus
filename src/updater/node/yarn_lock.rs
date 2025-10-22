use log::*;
use regex::Regex;
use std::collections::HashSet;

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
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
    pub async fn process_packages(
        &self,
        packages: &[(String, UpdaterPackage)],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];
        let mut processed_paths = HashSet::new();

        // First, handle workspace-level yarn.lock files
        let mut workspace_roots: Vec<&str> = packages
            .iter()
            .map(|(_, p)| p.workspace_root.as_str())
            .collect();
        workspace_roots.sort_unstable();
        workspace_roots.dedup();

        for workspace_root in workspace_roots {
            // Get a package from this workspace to use its helper method
            let workspace_package = packages
                .iter()
                .find(|(_, p)| p.workspace_root == workspace_root)
                .map(|(_, p)| p);

            if workspace_package.is_none() {
                continue;
            }

            let workspace_package = workspace_package.unwrap();
            let workspace_lock_path =
                workspace_package.get_workspace_file_path("yarn.lock");

            let workspace_packages: Vec<(String, UpdaterPackage)> = packages
                .iter()
                .filter(|(_, p)| p.workspace_root == workspace_root)
                .cloned()
                .collect();

            if let Some(change) = self
                .update_lock_file(
                    &workspace_lock_path,
                    &workspace_packages,
                    loader,
                )
                .await?
            {
                processed_paths.insert(change.path.clone());
                file_changes.push(change);
            }
        }

        // Then handle package-level yarn.lock files
        for (package_name, package) in packages.iter() {
            let path_str = package.get_file_path("yarn.lock");

            // Skip if this path was already processed as a workspace lock file
            if processed_paths.contains(&path_str) {
                continue;
            }

            if let Some(change) = self
                .update_lock_file(
                    &path_str,
                    &[(package_name.clone(), package.clone())],
                    loader,
                )
                .await?
            {
                file_changes.push(change);
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    /// Update a single yarn.lock file
    async fn update_lock_file(
        &self,
        lock_path: &str,
        all_packages: &[(String, UpdaterPackage)],
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        let content = loader.get_file_content(lock_path).await?;

        if content.is_none() {
            return Ok(None);
        }

        info!("Updating yarn.lock at {lock_path}");

        let content = content.unwrap();
        let mut lines: Vec<String> = vec![];

        // Regex to match package entries like "package@^1.0.0:"
        let package_regex = Regex::new(r#"^"?([^@"]+)@[^"]*"?:$"#)?;
        let version_regex = Regex::new(r#"^(\s+version\s+)"(.*)""#)?;

        let mut current_yarn_package: Option<String> = None;
        let mut skip_current_package = false;

        for line in content.lines() {
            // Check if this line starts a new package entry
            if let Some(caps) = package_regex.captures(line) {
                current_yarn_package = Some(caps[1].to_string());
                // Skip packages using workspace: or repo: protocols
                skip_current_package =
                    line.contains("workspace:") || line.contains("repo:");
                lines.push(line.to_string());
                continue;
            }

            // Check if this is a version line and we're in a relevant package
            if !skip_current_package
                && let (Some(pkg_name), Some(caps)) = (
                    current_yarn_package.as_ref(),
                    version_regex.captures(line),
                )
                && let Some((_, package)) =
                    all_packages.iter().find(|(n, _)| n == pkg_name)
            {
                let new_line =
                    format!("{}\"{}\"", &caps[1], package.next_version.semver);
                lines.push(new_line);
                continue;
            }

            // Reset current package when we hit an empty line or start of new entry
            if line.trim().is_empty()
                || (!line.starts_with(' ')
                    && !line.starts_with('\t')
                    && line.contains(':'))
            {
                current_yarn_package = None;
                skip_current_package = false;
            }

            lines.push(line.to_string());
        }

        let updated_content = lines.join("\n");

        Ok(Some(FileChange {
            path: lock_path.to_string(),
            content: updated_content,
            update_type: FileUpdateType::Replace,
        }))
    }
}
