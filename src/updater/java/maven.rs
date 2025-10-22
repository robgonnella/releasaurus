use log::*;
use quick_xml::events::{BytesText, Event};
use quick_xml::{Reader, Writer as XmlWriter};

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
    updater::framework::UpdaterPackage,
};

/// Handles Maven pom.xml file parsing and version updates for Java packages.
pub struct Maven {}

impl Maven {
    /// Create Maven handler for pom.xml version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version fields in pom.xml files for all Java packages.
    pub async fn process_packages(
        &self,
        packages: &[UpdaterPackage],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for package in packages {
            let pom_path = package.get_file_path("pom.xml");

            if let Some(change) =
                self.update_pom_file(&pom_path, package, loader).await?
            {
                file_changes.push(change);
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    /// Update a single pom.xml file
    async fn update_pom_file(
        &self,
        pom_path: &str,
        package: &UpdaterPackage,
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        let content = loader.get_file_content(pom_path).await?;

        if content.is_none() {
            return Ok(None);
        }

        info!("Updating Maven project: {}", package.path);

        let content = content.unwrap();
        let bytes = content.as_bytes();

        let mut reader = Reader::from_reader(bytes);

        let mut writer = XmlWriter::new(Vec::new());
        let mut in_project_version = false;
        let mut in_version_element = false;
        let mut depth = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    depth += 1;
                    if e.name().as_ref() == b"version" {
                        // Check if we're at the project level (depth should be 2: project > version)
                        if depth == 2 {
                            in_project_version = true;
                        }
                        in_version_element = true;
                    }
                    writer.write_event(Event::Start(e.clone()))?;
                }
                Ok(Event::End(ref e)) => {
                    depth -= 1;
                    if e.name().as_ref() == b"version" {
                        in_version_element = false;
                        if in_project_version {
                            in_project_version = false;
                        }
                    }
                    writer.write_event(Event::End(e.clone()))?;
                }
                Ok(Event::Text(ref e)) => {
                    if in_project_version && in_version_element {
                        // Replace the version text
                        let new_version =
                            package.next_version.semver.to_string();
                        info!("Updating Maven version to: {}", new_version);
                        writer.write_event(Event::Text(BytesText::new(
                            &new_version,
                        )))?;
                    } else {
                        writer.write_event(Event::Text(e.clone()))?;
                    }
                }
                Ok(Event::Eof) => break,
                Ok(e) => writer.write_event(e)?,
                Err(e) => return Err(e.into()),
            }
        }

        let result = writer.into_inner();
        let content = String::from_utf8(result)?;
        Ok(Some(FileChange {
            path: pom_path.to_string(),
            content,
            update_type: FileUpdateType::Replace,
        }))
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
    async fn test_update_maven_project() {
        let maven = Maven::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let pom_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>test-package</artifactId>
    <version>1.0.0</version>
</project>"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pom.xml"))
            .times(1)
            .returning({
                let content = pom_xml.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = maven
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/pom.xml");
        assert!(changes[0].content.contains("<version>2.0.0</version>"));
    }

    #[tokio::test]
    async fn test_update_maven_project_with_nested_versions() {
        let maven = Maven::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let pom_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>test-package</artifactId>
    <version>1.0.0</version>
    <dependencies>
        <dependency>
            <groupId>junit</groupId>
            <artifactId>junit</artifactId>
            <version>4.12</version>
        </dependency>
    </dependencies>
</project>"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pom.xml"))
            .times(1)
            .returning({
                let content = pom_xml.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = maven
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        // Should only update the project version, not dependency versions
        assert!(changes[0].content.contains("<version>2.0.0</version>"));
        assert!(changes[0].content.contains("<version>4.12</version>"));
    }

    #[tokio::test]
    async fn test_update_maven_project_file_not_found() {
        let maven = Maven::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pom.xml"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = maven
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }
}
