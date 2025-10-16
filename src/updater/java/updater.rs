use async_trait::async_trait;
use log::*;
use quick_xml::events::{BytesText, Event};
use quick_xml::{Reader, Writer as XmlWriter};
use regex::Regex;

use crate::{
    forge::request::{FileChange, FileUpdateType},
    forge::traits::FileLoader,
    result::Result,
    updater::framework::Framework,
    updater::{framework::UpdaterPackage, traits::PackageUpdater},
};

/// Java package updater supporting Maven and Gradle projects.
pub struct JavaUpdater {}

impl JavaUpdater {
    /// Create Java updater for Maven pom.xml and Gradle build files.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version numbers in Maven pom.xml or Gradle build files for Java
    /// packages.
    async fn process_packages(
        &self,
        packages: &[UpdaterPackage],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for package in packages {
            // Try Maven first (pom.xml)
            let pom_path = package.get_file_path("pom.xml");
            if let Some(change) = self
                .update_maven_project(&pom_path, package, loader)
                .await?
            {
                file_changes.push(change);
                continue;
            }

            // Try Gradle (build.gradle or build.gradle.kts)
            let gradle_path = package.get_file_path("build.gradle");
            if let Some(change) = self
                .update_gradle_project(&gradle_path, package, false, loader)
                .await?
            {
                file_changes.push(change);
            }

            let gradle_kts_path = package.get_file_path("build.gradle.kts");
            if let Some(change) = self
                .update_gradle_project(&gradle_kts_path, package, true, loader)
                .await?
            {
                file_changes.push(change);
            }

            // Also check for gradle.properties
            let gradle_props_path = package.get_file_path("gradle.properties");

            if let Some(change) = self
                .update_gradle_properties(&gradle_props_path, package, loader)
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

    /// Update Maven project by modifying pom.xml
    async fn update_maven_project(
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

    /// Update Gradle project by modifying build.gradle or build.gradle.kts
    async fn update_gradle_project(
        &self,
        build_path: &str,
        package: &UpdaterPackage,
        is_kotlin: bool,
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        let content = loader.get_file_content(build_path).await?;

        if content.is_none() {
            return Ok(None);
        }

        info!("Updating Gradle project: {}", build_path);

        let content = content.unwrap();

        let new_version = package.next_version.semver.to_string();

        // Define regex patterns for different version declaration styles
        let patterns = if is_kotlin {
            vec![
                // Kotlin DSL patterns
                Regex::new(r#"version\s*=\s*"[^"]*""#)?,
                Regex::new(r#"version\s*=\s*'[^']*'"#)?,
                Regex::new(r#"val\s+version\s*=\s*"[^"]*""#)?,
                Regex::new(r#"val\s+version\s*=\s*'[^']*'"#)?,
                Regex::new(r#"project\.version\s*=\s*"[^"]*""#)?,
                Regex::new(r#"project\.version\s*=\s*'[^']*'"#)?,
            ]
        } else {
            vec![
                // Groovy DSL patterns
                Regex::new(r#"version\s*=\s*["'][^"']*["']"#)?,
                Regex::new(r#"version\s+["'][^"']*["']"#)?,
                Regex::new(r#"def\s+version\s*=\s*["'][^"']*["']"#)?,
                Regex::new(r#"project\.version\s*=\s*["'][^"']*["']"#)?,
            ]
        };

        let mut updated_content = content.clone();
        let mut version_found = false;

        for pattern in patterns {
            if pattern.is_match(&content) {
                if is_kotlin {
                    updated_content = pattern
                        .replace_all(
                            &updated_content,
                            |caps: &regex::Captures| {
                                let full_match = caps.get(0).unwrap().as_str();
                                if full_match.contains('"') {
                                    format!(
                                        "{}\"{}\"",
                                        full_match
                                            .split('"')
                                            .next()
                                            .unwrap_or(""),
                                        new_version
                                    )
                                } else {
                                    format!(
                                        "{}'{}\'",
                                        full_match
                                            .split('\'')
                                            .next()
                                            .unwrap_or(""),
                                        new_version
                                    )
                                }
                            },
                        )
                        .to_string();
                } else {
                    updated_content = pattern
                        .replace_all(
                            &updated_content,
                            |caps: &regex::Captures| {
                                let full_match = caps.get(0).unwrap().as_str();
                                let quote_char = if full_match.contains('"') {
                                    '"'
                                } else {
                                    '\''
                                };
                                let prefix = full_match
                                    .split(quote_char)
                                    .next()
                                    .unwrap_or("");
                                format!(
                                    "{}{}{}{}",
                                    prefix, quote_char, new_version, quote_char
                                )
                            },
                        )
                        .to_string();
                }
                version_found = true;
                break;
            }
        }

        if version_found {
            info!("Updating Gradle version to: {}", new_version);
            return Ok(Some(FileChange {
                path: build_path.to_string(),
                content: updated_content,
                update_type: FileUpdateType::Replace,
            }));
        }

        info!(
            "No version declaration found in Gradle build file: {build_path}",
        );
        Ok(None)
    }

    /// Update gradle.properties file
    async fn update_gradle_properties(
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
        for line in content.lines() {
            if line.trim_start().starts_with("version") && line.contains('=') {
                lines.push(format!("version={}", new_version));
                version_updated = true;
                info!(
                    "Updated version in gradle.properties to: {}",
                    new_version
                );
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

        info!("Found {} Java packages", java_packages.len());

        if java_packages.is_empty() {
            return Ok(None);
        }

        self.process_packages(&java_packages, loader).await
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
    async fn test_update_maven_project() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "2.0.0",
            Framework::Java,
        );

        let pom_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>test-project</artifactId>
    <version>1.0.0</version>
    <name>Test Project</name>
</project>"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/pom.xml"))
            .times(1)
            .returning(move |_| Ok(Some(pom_content.to_string())));

        let result = updater
            .update_maven_project(
                "test-project/pom.xml",
                &package,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.path, "test-project/pom.xml");
        assert!(change.content.contains("<version>2.0.0</version>"));
        assert!(!change.content.contains("<version>1.0.0</version>"));
    }

    #[tokio::test]
    async fn test_update_maven_project_with_nested_versions() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "3.0.0",
            Framework::Java,
        );

        let pom_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>test-project</artifactId>
    <version>1.0.0</version>
    <dependencies>
        <dependency>
            <groupId>org.example</groupId>
            <artifactId>some-lib</artifactId>
            <version>5.0.0</version>
        </dependency>
    </dependencies>
</project>"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/pom.xml"))
            .times(1)
            .returning(move |_| Ok(Some(pom_content.to_string())));

        let result = updater
            .update_maven_project(
                "test-project/pom.xml",
                &package,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        // Should only update project version, not dependency version
        assert!(change.content.contains("<version>3.0.0</version>"));
        assert!(change.content.contains("<version>5.0.0</version>"));
        assert!(!change.content.contains("<version>1.0.0</version>"));
    }

    #[tokio::test]
    async fn test_update_maven_project_file_not_found() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "2.0.0",
            Framework::Java,
        );

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/pom.xml"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater
            .update_maven_project(
                "test-project/pom.xml",
                &package,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_gradle_project_groovy_double_quotes() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "2.0.0",
            Framework::Java,
        );

        let gradle_content = r#"plugins {
    id 'java'
}

group = 'com.example'
version = "1.0.0"

repositories {
    mavenCentral()
}
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/build.gradle"))
            .times(1)
            .returning(move |_| Ok(Some(gradle_content.to_string())));

        let result = updater
            .update_gradle_project(
                "test-project/build.gradle",
                &package,
                false,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.path, "test-project/build.gradle");
        assert!(change.content.contains(r#"version = "2.0.0""#));
        assert!(!change.content.contains(r#"version = "1.0.0""#));
    }

    #[tokio::test]
    async fn test_update_gradle_project_groovy_single_quotes() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "2.5.0",
            Framework::Java,
        );

        let gradle_content = r#"plugins {
    id 'java'
}

group = 'com.example'
version = '1.0.0'

repositories {
    mavenCentral()
}
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/build.gradle"))
            .times(1)
            .returning(move |_| Ok(Some(gradle_content.to_string())));

        let result = updater
            .update_gradle_project(
                "test-project/build.gradle",
                &package,
                false,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        assert!(change.content.contains(r#"version = '2.5.0'"#));
        assert!(!change.content.contains(r#"version = '1.0.0'"#));
    }

    #[tokio::test]
    async fn test_update_gradle_project_kotlin_dsl() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "3.0.0",
            Framework::Java,
        );

        let gradle_content = r#"plugins {
    kotlin("jvm") version "1.9.0"
}

group = "com.example"
version = "1.0.0"

repositories {
    mavenCentral()
}
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/build.gradle.kts"))
            .times(1)
            .returning(move |_| Ok(Some(gradle_content.to_string())));

        let result = updater
            .update_gradle_project(
                "test-project/build.gradle.kts",
                &package,
                true,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.path, "test-project/build.gradle.kts");
        assert!(change.content.contains(r#"version = "3.0.0""#));
        assert!(!change.content.contains(r#"version = "1.0.0""#));
    }

    #[tokio::test]
    async fn test_update_gradle_project_kotlin_val_declaration() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "4.0.0",
            Framework::Java,
        );

        let gradle_content = r#"plugins {
    kotlin("jvm") version "1.9.0"
}

val version = "1.0.0"

repositories {
    mavenCentral()
}
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/build.gradle.kts"))
            .times(1)
            .returning(move |_| Ok(Some(gradle_content.to_string())));

        let result = updater
            .update_gradle_project(
                "test-project/build.gradle.kts",
                &package,
                true,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        assert!(change.content.contains(r#"val version = "4.0.0""#));
        assert!(!change.content.contains(r#"val version = "1.0.0""#));
    }

    #[tokio::test]
    async fn test_update_gradle_project_no_version() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "2.0.0",
            Framework::Java,
        );

        let gradle_content = r#"plugins {
    id 'java'
}

group = 'com.example'

repositories {
    mavenCentral()
}
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/build.gradle"))
            .times(1)
            .returning(move |_| Ok(Some(gradle_content.to_string())));

        let result = updater
            .update_gradle_project(
                "test-project/build.gradle",
                &package,
                false,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_gradle_properties() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "2.0.0",
            Framework::Java,
        );

        let props_content = r#"# Project properties
version=1.0.0
group=com.example
name=test-project
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/gradle.properties"))
            .times(1)
            .returning(move |_| Ok(Some(props_content.to_string())));

        let result = updater
            .update_gradle_properties(
                "test-project/gradle.properties",
                &package,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.path, "test-project/gradle.properties");
        assert!(change.content.contains("version=2.0.0"));
        assert!(!change.content.contains("version=1.0.0"));
        assert!(change.content.contains("group=com.example"));
        assert!(change.content.contains("name=test-project"));
    }

    #[tokio::test]
    async fn test_update_gradle_properties_with_spaces() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "3.0.0",
            Framework::Java,
        );

        let props_content = r#"# Project properties
  version=1.0.0
group=com.example
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/gradle.properties"))
            .times(1)
            .returning(move |_| Ok(Some(props_content.to_string())));

        let result = updater
            .update_gradle_properties(
                "test-project/gradle.properties",
                &package,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        assert!(change.content.contains("version=3.0.0"));
    }

    #[tokio::test]
    async fn test_update_gradle_properties_no_version() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "2.0.0",
            Framework::Java,
        );

        let props_content = r#"# Project properties
group=com.example
name=test-project
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/gradle.properties"))
            .times(1)
            .returning(move |_| Ok(Some(props_content.to_string())));

        let result = updater
            .update_gradle_properties(
                "test-project/gradle.properties",
                &package,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_gradle_properties_file_not_found() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "2.0.0",
            Framework::Java,
        );

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/gradle.properties"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater
            .update_gradle_properties(
                "test-project/gradle.properties",
                &package,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_filters_java_packages() {
        let updater = JavaUpdater::new();

        let packages = vec![
            create_test_updater_package(
                "test-package",
                "java-project",
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
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        // Should process but find no files (returns None because no changes made)
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_gradle_project_with_project_prefix() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "2.0.0",
            Framework::Java,
        );

        let gradle_content = r#"plugins {
    id 'java'
}

group = 'com.example'
project.version = "1.0.0"

repositories {
    mavenCentral()
}
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/build.gradle"))
            .times(1)
            .returning(move |_| Ok(Some(gradle_content.to_string())));

        let result = updater
            .update_gradle_project(
                "test-project/build.gradle",
                &package,
                false,
                &mock_loader,
            )
            .await
            .unwrap();

        assert!(result.is_some());
        let change = result.unwrap();
        assert!(change.content.contains(r#"project.version = "2.0.0""#));
        assert!(!change.content.contains(r#"project.version = "1.0.0""#));
    }

    #[tokio::test]
    async fn test_process_packages_with_multiple_gradle_files() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "2.0.0",
            Framework::Java,
        );

        let gradle_content = r#"version = "1.0.0""#;
        let props_content = r#"version=1.0.0"#;

        let mut mock_loader = MockFileLoader::new();

        // pom.xml doesn't exist
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/pom.xml"))
            .times(1)
            .returning(|_| Ok(None));

        // build.gradle exists and has version
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/build.gradle"))
            .times(1)
            .returning({
                let content = gradle_content.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // build.gradle.kts doesn't exist
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/build.gradle.kts"))
            .times(1)
            .returning(|_| Ok(None));

        // gradle.properties exists and has version
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/gradle.properties"))
            .times(1)
            .returning({
                let content = props_content.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = updater
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);

        // Verify build.gradle was updated
        assert!(changes.iter().any(|c| c.path == "test-project/build.gradle"
            && c.content.contains(r#"version = "2.0.0""#)));

        // Verify gradle.properties was updated
        assert!(
            changes
                .iter()
                .any(|c| c.path == "test-project/gradle.properties"
                    && c.content.contains("version=2.0.0"))
        );
    }

    #[tokio::test]
    async fn test_process_packages_maven_takes_precedence() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "3.0.0",
            Framework::Java,
        );

        let pom_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <version>1.0.0</version>
</project>"#;
        let mut mock_loader = MockFileLoader::new();

        // pom.xml exists
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("test-project/pom.xml"))
            .times(1)
            .returning({
                let content = pom_content.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // build.gradle should NOT be called because Maven takes precedence
        // (the continue statement in process_packages)

        let packages = vec![package];
        let result = updater
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);

        // Only Maven file should be updated
        assert_eq!(changes[0].path, "test-project/pom.xml");
        assert!(changes[0].content.contains("<version>3.0.0</version>"));
    }

    #[tokio::test]
    async fn test_process_packages_no_files_found() {
        let updater = JavaUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "test-project",
            "2.0.0",
            Framework::Java,
        );

        let mut mock_loader = MockFileLoader::new();

        // All files return None (don't exist)
        mock_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        // Should return None when no files are found
        assert!(result.is_none());
    }
}
