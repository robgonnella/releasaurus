use async_trait::async_trait;
use log::*;

use crate::{
    forge::request::FileChange,
    forge::traits::FileLoader,
    result::Result,
    updater::framework::Framework,
    updater::{framework::UpdaterPackage, traits::PackageUpdater},
};

use super::gradle::Gradle;
use super::gradle_properties::GradleProperties;
use super::maven::Maven;

/// Java package updater supporting Maven and Gradle projects.
pub struct JavaUpdater {
    maven: Maven,
    gradle: Gradle,
    gradle_properties: GradleProperties,
}

impl JavaUpdater {
    /// Create Java updater for Maven pom.xml and Gradle build files.
    pub fn new() -> Self {
        Self {
            maven: Maven::new(),
            gradle: Gradle::new(),
            gradle_properties: GradleProperties::new(),
        }
    }
}

#[async_trait]
impl PackageUpdater for JavaUpdater {
    async fn update(
        &self,
        packages: Vec<UpdaterPackage>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let java_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Java))
            .collect::<Vec<UpdaterPackage>>();

        info!("Found {} java packages", java_packages.len());

        if java_packages.is_empty() {
            return Ok(None);
        }

        let mut file_changes: Vec<FileChange> = vec![];

        // Try Maven first (pom.xml) - takes precedence
        if let Some(changes) =
            self.maven.process_packages(&java_packages, loader).await?
        {
            file_changes.extend(changes);
        }

        // Try Gradle build files (build.gradle, build.gradle.kts)
        if let Some(changes) =
            self.gradle.process_packages(&java_packages, loader).await?
        {
            file_changes.extend(changes);
        }

        // Try gradle.properties
        if let Some(changes) = self
            .gradle_properties
            .process_packages(&java_packages, loader)
            .await?
        {
            file_changes.extend(changes);
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::release::Tag;
    use crate::forge::traits::MockFileLoader;
    use crate::test_helpers::create_test_updater_package;
    use semver::Version as SemVer;

    #[tokio::test]
    async fn test_update_maven_project_file_not_found() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
            Framework::Java,
        );

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pom.xml"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/build.gradle"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/build.gradle.kts"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/gradle.properties"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_filters_java_packages() {
        let updater = JavaUpdater::new();

        let packages = vec![
            create_test_updater_package(
                "java-package",
                "packages/java",
                "2.0.0",
                Framework::Java,
            ),
            UpdaterPackage {
                name: "node-project".to_string(),
                path: "node-project".to_string(),
                workspace_root: ".".into(),
                framework: Framework::Node,
                next_version: Tag {
                    sha: "test-sha".to_string(),
                    name: "v1.0.0".to_string(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                },
            },
        ];

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/java/pom.xml"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/java/build.gradle"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/java/build.gradle.kts"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/java/gradle.properties"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_process_packages_maven_takes_precedence() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
            Framework::Java,
        );

        let pom_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <version>1.0.0</version>
</project>"#;

        let build_gradle = r#"
version = "1.0.0"
"#;

        let mut mock_loader = MockFileLoader::new();
        // Maven file exists and will be processed
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pom.xml"))
            .times(1)
            .returning({
                let content = pom_xml.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Gradle files should still be checked since we process all files now
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/build.gradle"))
            .times(1)
            .returning({
                let content = build_gradle.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/build.gradle.kts"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/gradle.properties"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        // Should update both pom.xml and build.gradle
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().any(|c| c.path == "packages/test/pom.xml"));
        assert!(
            changes
                .iter()
                .any(|c| c.path == "packages/test/build.gradle")
        );
    }

    #[tokio::test]
    async fn test_process_packages_with_multiple_gradle_files() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
            Framework::Java,
        );

        let build_gradle = r#"version = "1.0.0""#;
        let build_gradle_kts = r#"version = "1.0.0""#;
        let gradle_properties = r#"version=1.0.0"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pom.xml"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/build.gradle"))
            .times(1)
            .returning({
                let content = build_gradle.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/build.gradle.kts"))
            .times(1)
            .returning({
                let content = build_gradle_kts.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/gradle.properties"))
            .times(1)
            .returning({
                let content = gradle_properties.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        // Should update all three files
        assert_eq!(changes.len(), 3);
        assert!(
            changes
                .iter()
                .any(|c| c.path == "packages/test/build.gradle")
        );
        assert!(
            changes
                .iter()
                .any(|c| c.path == "packages/test/build.gradle.kts")
        );
        assert!(
            changes
                .iter()
                .any(|c| c.path == "packages/test/gradle.properties")
        );
    }
}
