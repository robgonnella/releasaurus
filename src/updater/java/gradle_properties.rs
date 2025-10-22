use log::*;
use regex::Regex;

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
    updater::framework::UpdaterPackage,
};

/// Handles gradle.properties file parsing and version updates for Java packages.
pub struct GradleProperties {}

impl GradleProperties {
    /// Create GradleProperties handler for gradle.properties version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version fields in gradle.properties files for all Java packages.
    pub async fn process_packages(
        &self,
        packages: &[UpdaterPackage],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for package in packages {
            let props_path = package.get_file_path("gradle.properties");

            if let Some(change) = self
                .update_properties_file(&props_path, package, loader)
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

    /// Update a single gradle.properties file
    async fn update_properties_file(
        &self,
        props_path: &str,
        package: &UpdaterPackage,
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        let content = loader.get_file_content(props_path).await?;

        if content.is_none() {
            return Ok(None);
        }

        info!("Updating gradle.properties: {}", props_path);

        let content = content.unwrap();

        let mut lines: Vec<String> = Vec::new();
        let mut version_updated = false;

        let new_version = package.next_version.semver.to_string();

        // Read all lines and update version property
        // Regex to capture: indentation, "version", spacing around =, and old version
        let version_regex = Regex::new(r"^(\s*version\s*=\s*)(.*)$").unwrap();

        for line in content.lines() {
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
                path: props_path.to_string(),
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
    use crate::analyzer::release::Tag;
    use crate::forge::traits::MockFileLoader;
    use crate::updater::framework::{Framework, UpdaterPackage};
    use semver::Version as SemVer;

    fn create_test_updater_package(
        name: &str,
        path: &str,
        version: &str,
    ) -> UpdaterPackage {
        UpdaterPackage {
            name: name.to_string(),
            path: path.to_string(),
            workspace_root: ".".into(),
            framework: Framework::Java,
            next_version: Tag {
                sha: "test-sha".to_string(),
                name: format!("v{}", version),
                semver: SemVer::parse(version).unwrap(),
            },
        }
    }

    #[tokio::test]
    async fn test_update_gradle_properties() {
        let gradle_props = GradleProperties::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let gradle_properties = r#"
# Project properties
version=1.0.0
group=com.example
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/gradle.properties"))
            .times(1)
            .returning({
                let content = gradle_properties.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = gradle_props
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/gradle.properties");
        assert!(changes[0].content.contains("version=2.0.0"));
    }

    #[tokio::test]
    async fn test_update_gradle_properties_with_spaces() {
        let gradle_props = GradleProperties::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let gradle_properties = r#"
# Project properties
version = 1.0.0
group = com.example
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/gradle.properties"))
            .times(1)
            .returning({
                let content = gradle_properties.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = gradle_props
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].content.contains("version = 2.0.0"));
    }

    #[tokio::test]
    async fn test_update_gradle_properties_no_version() {
        let gradle_props = GradleProperties::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let gradle_properties = r#"
# Project properties
group=com.example
description=Test project
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/gradle.properties"))
            .times(1)
            .returning({
                let content = gradle_properties.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = gradle_props
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_gradle_properties_file_not_found() {
        let gradle_props = GradleProperties::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/gradle.properties"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = gradle_props
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_gradle_properties_preserves_no_spacing() {
        let gradle_props = GradleProperties::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let gradle_properties = r#"
# Project properties
version=1.0.0
group=com.example
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/gradle.properties"))
            .times(1)
            .returning({
                let content = gradle_properties.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = gradle_props
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        // Should preserve no spaces around equals
        assert!(changes[0].content.contains("version=2.0.0"));
    }

    #[tokio::test]
    async fn test_gradle_properties_preserves_multiple_spaces() {
        let gradle_props = GradleProperties::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let gradle_properties = r#"
# Project properties
version  =  1.0.0
group = com.example
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/gradle.properties"))
            .times(1)
            .returning({
                let content = gradle_properties.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = gradle_props
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        // Should preserve multiple spaces around equals
        assert!(changes[0].content.contains("version  =  2.0.0"));
    }

    #[tokio::test]
    async fn test_gradle_properties_preserves_indentation() {
        let gradle_props = GradleProperties::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let gradle_properties = r#"
# Project properties
  version = 1.0.0
  group = com.example
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/gradle.properties"))
            .times(1)
            .returning({
                let content = gradle_properties.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = gradle_props
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        // Should preserve leading spaces
        assert!(changes[0].content.contains("  version = 2.0.0"));
    }
}
