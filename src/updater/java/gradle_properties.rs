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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_helpers::create_test_tag,
        updater::framework::{Framework, ManifestFile, UpdaterPackage},
    };

    #[tokio::test]
    async fn updates_version_property() {
        let properties = GradleProperties::new();
        let content = "version=1.0.0";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "gradle.properties".to_string(),
            file_basename: "gradle.properties".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = properties
            .update_properties_file(&manifest, &package)
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().content, "version=2.0.0");
    }

    #[tokio::test]
    async fn preserves_whitespace_around_equals() {
        let properties = GradleProperties::new();
        let content = "version  =  1.0.0";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "gradle.properties".to_string(),
            file_basename: "gradle.properties".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v3.0.0", "3.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = properties
            .update_properties_file(&manifest, &package)
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().content, "version  =  3.0.0");
    }

    #[tokio::test]
    async fn preserves_leading_whitespace() {
        let properties = GradleProperties::new();
        let content = "  version=1.0.0";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "gradle.properties".to_string(),
            file_basename: "gradle.properties".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.5.0", "2.5.0", "abc"),
            framework: Framework::Java,
        };

        let result = properties
            .update_properties_file(&manifest, &package)
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().content, "  version=2.5.0");
    }

    #[tokio::test]
    async fn preserves_other_properties() {
        let properties = GradleProperties::new();
        let content =
            "org.gradle.jvmargs=-Xmx2048m\nversion=1.0.0\ngroup=com.example";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "gradle.properties".to_string(),
            file_basename: "gradle.properties".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = properties
            .update_properties_file(&manifest, &package)
            .await
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap().content;
        assert!(updated.contains("org.gradle.jvmargs=-Xmx2048m"));
        assert!(updated.contains("version=2.0.0"));
        assert!(updated.contains("group=com.example"));
    }

    #[tokio::test]
    async fn returns_none_when_no_version_property() {
        let properties = GradleProperties::new();
        let content = "org.gradle.jvmargs=-Xmx2048m\ngroup=com.example";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "gradle.properties".to_string(),
            file_basename: "gradle.properties".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = properties
            .update_properties_file(&manifest, &package)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn process_package_handles_multiple_properties_files() {
        let properties = GradleProperties::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            file_path: "module1/gradle.properties".to_string(),
            file_basename: "gradle.properties".to_string(),
            content: "version=1.0.0".to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            file_path: "module2/gradle.properties".to_string(),
            file_basename: "gradle.properties".to_string(),
            content: "version=1.0.0".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = properties.process_package(&package).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[tokio::test]
    async fn process_package_returns_none_when_no_gradle_properties() {
        let properties = GradleProperties::new();
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "build.gradle".to_string(),
            file_basename: "build.gradle".to_string(),
            content: "version = \"1.0.0\"".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = properties.process_package(&package).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn ignores_commented_version_lines() {
        let properties = GradleProperties::new();
        let content = "# version=0.0.1\nversion=1.0.0";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "gradle.properties".to_string(),
            file_basename: "gradle.properties".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Java,
        };

        let result = properties
            .update_properties_file(&manifest, &package)
            .await
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap().content;
        assert!(updated.contains("# version=0.0.1"));
        assert!(updated.contains("version=2.0.0"));
    }
}
