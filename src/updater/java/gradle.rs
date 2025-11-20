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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_helpers::create_test_tag,
        updater::framework::{Framework, ManifestFile, UpdaterPackage},
    };

    #[tokio::test]
    async fn updates_groovy_version_with_double_quotes() {
        let gradle = Gradle::new();
        let content = r#"version = "1.0.0""#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "build.gradle".to_string(),
            file_basename: "build.gradle".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = gradle
            .update_gradle_file(&manifest, &package, false)
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.content, r#"version = "2.0.0""#);
    }

    #[tokio::test]
    async fn updates_groovy_version_with_single_quotes() {
        let gradle = Gradle::new();
        let content = "version = '1.0.0'";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "build.gradle".to_string(),
            file_basename: "build.gradle".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = gradle
            .update_gradle_file(&manifest, &package, false)
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().content, "version = '2.0.0'");
    }

    #[tokio::test]
    async fn updates_kotlin_version() {
        let gradle = Gradle::new();
        let content = r#"version = "1.0.0""#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "build.gradle.kts".to_string(),
            file_basename: "build.gradle.kts".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v3.5.0", "3.5.0", "abc"),
            framework: Framework::Java,
        };

        let result = gradle
            .update_gradle_file(&manifest, &package, true)
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().content, r#"version = "3.5.0""#);
    }

    #[tokio::test]
    async fn updates_project_version_declaration() {
        let gradle = Gradle::new();
        let content = r#"project.version = "1.0.0""#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "build.gradle".to_string(),
            file_basename: "build.gradle".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v4.0.0", "4.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = gradle
            .update_gradle_file(&manifest, &package, false)
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().content, r#"project.version = "4.0.0""#);
    }

    #[tokio::test]
    async fn returns_none_when_no_version_found() {
        let gradle = Gradle::new();
        let content = "dependencies { implementation 'com.example:lib:1.0.0' }";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "build.gradle".to_string(),
            file_basename: "build.gradle".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = gradle
            .update_gradle_file(&manifest, &package, false)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn process_package_handles_multiple_manifests() {
        let gradle = Gradle::new();
        let groovy_manifest = ManifestFile {
            is_workspace: false,
            file_path: "build.gradle".to_string(),
            file_basename: "build.gradle".to_string(),
            content: r#"version = "1.0.0""#.to_string(),
        };
        let kotlin_manifest = ManifestFile {
            is_workspace: false,
            file_path: "build.gradle.kts".to_string(),
            file_basename: "build.gradle.kts".to_string(),
            content: r#"version = "1.0.0""#.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![groovy_manifest, kotlin_manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = gradle.process_package(&package).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[tokio::test]
    async fn process_package_returns_none_when_no_changes() {
        let gradle = Gradle::new();
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "pom.xml".to_string(),
            file_basename: "pom.xml".to_string(),
            content: "<version>1.0.0</version>".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = gradle.process_package(&package).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn preserves_whitespace_formatting() {
        let gradle = Gradle::new();
        let content = "version   =   \"1.0.0\"";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "build.gradle".to_string(),
            file_basename: "build.gradle".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = gradle
            .update_gradle_file(&manifest, &package, false)
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().content, "version   =   \"2.0.0\"");
    }
}
