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

/// Handles Gradle build.gradle and build.gradle.kts file parsing and version updates for Java packages.
pub struct Gradle {}

impl Gradle {
    /// Create Gradle handler for build file version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version fields in Gradle build files for all Java packages.
    pub async fn process_packages(
        &self,
        packages: &[UpdaterPackage],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for package in packages {
            // Try Groovy DSL (build.gradle)
            let gradle_path = package.get_file_path("build.gradle");
            if let Some(change) = self
                .update_gradle_file(&gradle_path, package, false, loader)
                .await?
            {
                file_changes.push(change);
            }

            // Try Kotlin DSL (build.gradle.kts)
            let gradle_kts_path = package.get_file_path("build.gradle.kts");
            if let Some(change) = self
                .update_gradle_file(&gradle_kts_path, package, true, loader)
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

    /// Update a single Gradle build file (build.gradle or build.gradle.kts)
    async fn update_gradle_file(
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
    async fn test_update_gradle_project_groovy_double_quotes() {
        let gradle = Gradle::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let build_gradle = r#"
plugins {
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

        let packages = vec![package];
        let result = gradle
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/build.gradle");
        assert!(changes[0].content.contains(r#"version = "2.0.0""#));
    }

    #[tokio::test]
    async fn test_update_gradle_project_groovy_single_quotes() {
        let gradle = Gradle::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let build_gradle = r#"
plugins {
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

        let packages = vec![package];
        let result = gradle
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/build.gradle");
        assert!(changes[0].content.contains(r#"version = '2.0.0'"#));
    }

    #[tokio::test]
    async fn test_update_gradle_project_kotlin_dsl() {
        let gradle = Gradle::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let build_gradle_kts = r#"
plugins {
    java
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
            .with(mockall::predicate::eq("packages/test/build.gradle"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/build.gradle.kts"))
            .times(1)
            .returning({
                let content = build_gradle_kts.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = gradle
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/build.gradle.kts");
        assert!(changes[0].content.contains(r#"version = "2.0.0""#));
    }

    #[tokio::test]
    async fn test_update_gradle_project_kotlin_val_declaration() {
        let gradle = Gradle::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let build_gradle_kts = r#"
plugins {
    java
}

val version = "1.0.0"
group = "com.example"

repositories {
    mavenCentral()
}
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/build.gradle"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/build.gradle.kts"))
            .times(1)
            .returning({
                let content = build_gradle_kts.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = gradle
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/build.gradle.kts");
        assert!(changes[0].content.contains(r#"val version = "2.0.0""#));
    }

    #[tokio::test]
    async fn test_update_gradle_project_no_version() {
        let gradle = Gradle::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let build_gradle = r#"
plugins {
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

        let packages = vec![package];
        let result = gradle
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        // Should return None if no version was found/updated
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_gradle_project_with_project_prefix() {
        let gradle = Gradle::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
        );

        let build_gradle = r#"
plugins {
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

        let packages = vec![package];
        let result = gradle
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/build.gradle");
        assert!(changes[0].content.contains(r#"project.version = "2.0.0""#));
    }
}
