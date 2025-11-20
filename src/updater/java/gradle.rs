use log::*;
use regex::Regex;

use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::{ManifestFile, UpdaterPackage},
};

/// Handles Gradle build.gradle and build.gradle.kts file parsing and version updates for Java packages.
pub struct Gradle {}

impl Gradle {
    /// Create Gradle handler for build file version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version fields in Gradle build files for all Java packages.
    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename == "build.gradle"
                && let Some(change) =
                    self.update_gradle_file(manifest, package, false).await?
            {
                file_changes.push(change);
            }

            if manifest.file_basename == "build.gradle.kts"
                && let Some(change) =
                    self.update_gradle_file(manifest, package, true).await?
            {
                file_changes.push(change);
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    /// Update a single Gradle build file (build.gradle or build.gradle.kts)
    async fn update_gradle_file(
        &self,
        manifest: &ManifestFile,
        package: &UpdaterPackage,
        is_kotlin: bool,
    ) -> Result<Option<FileChange>> {
        info!("Updating Gradle project: {}", manifest.file_path);

        let new_version = package.next_version.semver.to_string();

        // Define regex patterns for different version declaration styles
        // Each pattern uses capture groups to preserve formatting:
        // Group 1: prefix (everything before the version string)
        // Group 2: opening quote
        // Group 3: version value (to be replaced)
        // Group 4: closing quote
        let patterns = if is_kotlin {
            vec![
                // Kotlin DSL patterns
                Regex::new(r#"(version\s*=\s*)(")([^"]*)(")"#)?,
                Regex::new(r#"(version\s*=\s*)(')([^']*)(')"#)?,
                Regex::new(r#"(val\s+version\s*=\s*)(")([^"]*)(")"#)?,
                Regex::new(r#"(val\s+version\s*=\s*)(')([^']*)(')"#)?,
                Regex::new(r#"(project\.version\s*=\s*)(")([^"]*)(")"#)?,
                Regex::new(r#"(project\.version\s*=\s*)(')([^']*)(')"#)?,
            ]
        } else {
            vec![
                // Groovy DSL patterns
                Regex::new(r#"(version\s*=\s*)(")([^"]*)(")"#)?,
                Regex::new(r#"(version\s*=\s*)(')([^']*)(')"#)?,
                Regex::new(r#"(version\s+)(")([^"]*)(")"#)?,
                Regex::new(r#"(version\s+)(')([^']*)(')"#)?,
                Regex::new(r#"(def\s+version\s*=\s*)(")([^"]*)(")"#)?,
                Regex::new(r#"(def\s+version\s*=\s*)(')([^']*)(')"#)?,
                Regex::new(r#"(project\.version\s*=\s*)(")([^"]*)(")"#)?,
                Regex::new(r#"(project\.version\s*=\s*)(')([^']*)(')"#)?,
            ]
        };

        let mut updated_content = manifest.content.clone();
        let mut version_found = false;

        for pattern in patterns {
            if pattern.is_match(&manifest.content) {
                // Use capture groups to preserve formatting
                // $1 = prefix, $2 = opening quote, $3 = old version, $4 = closing quote
                updated_content = pattern
                    .replace_all(&updated_content, |caps: &regex::Captures| {
                        format!(
                            "{}{}{}{}",
                            &caps[1], &caps[2], new_version, &caps[4]
                        )
                    })
                    .to_string();
                version_found = true;
                break;
            }
        }

        if version_found {
            info!("Updating Gradle version to: {}", new_version);
            return Ok(Some(FileChange {
                path: manifest.file_path.clone(),
                content: updated_content,
                update_type: FileUpdateType::Replace,
            }));
        }

        info!(
            "No version declaration found in Gradle build file: {}",
            manifest.file_path
        );
        Ok(None)
    }
}
