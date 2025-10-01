use async_trait::async_trait;
use log::*;
use quick_xml::events::{BytesText, Event};
use quick_xml::{Reader, Writer as XmlWriter};
use regex::Regex;
use std::path::Path;

use crate::{
    forge::request::{FileChange, FileUpdateType},
    forge::traits::FileLoader,
    result::Result,
    updater::framework::Framework,
    updater::{framework::Package, traits::PackageUpdater},
};

/// Java package updater supporting Maven and Gradle projects.
pub struct JavaUpdater {}

impl JavaUpdater {
    pub fn new() -> Self {
        Self {}
    }

    /// Process packages and update their build files
    async fn process_packages(
        &self,
        packages: &[Package],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for package in packages {
            let package_path = Path::new(&package.path);

            // Try Maven first (pom.xml)
            let pom_path = package_path.join("pom.xml");
            if pom_path.exists() {
                let pom_path = pom_path.display().to_string();
                info!("Updating Maven project: {}", package.path);
                if let Some(change) = self
                    .update_maven_project(&pom_path, package, loader)
                    .await?
                {
                    file_changes.push(change);
                }
                continue;
            }

            // Try Gradle (build.gradle or build.gradle.kts)
            let gradle_path = package_path.join("build.gradle");
            let gradle_kts_path = package_path.join("build.gradle.kts");

            if gradle_path.exists() {
                let gradle_path = gradle_path.display().to_string();
                info!("Updating Gradle project: {}", gradle_path);
                if let Some(change) = self
                    .update_gradle_project(&gradle_path, package, false, loader)
                    .await?
                {
                    file_changes.push(change);
                }
            } else if gradle_kts_path.exists() {
                let gradle_kts_path = gradle_kts_path.display().to_string();
                info!("Updating Gradle Kotlin DSL project: {}", package.path);
                if let Some(change) = self
                    .update_gradle_project(
                        &gradle_kts_path,
                        package,
                        true,
                        loader,
                    )
                    .await?
                {
                    file_changes.push(change);
                }
            } else {
                info!(
                    "No Maven or Gradle build file found for package: {}",
                    package.path
                );
            }

            // Also check for gradle.properties
            let gradle_props_path = package_path.join("gradle.properties");
            if gradle_props_path.exists() {
                let gradle_props_path = gradle_props_path.display().to_string();
                if let Some(change) = self
                    .update_gradle_properties(
                        &gradle_props_path,
                        package,
                        loader,
                    )
                    .await?
                {
                    file_changes.push(change);
                }
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
        package: &Package,
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        let content = loader.get_file_content(pom_path).await?;

        if content.is_none() {
            return Ok(None);
        }

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
        package: &Package,
        is_kotlin: bool,
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        let content = loader.get_file_content(build_path).await?;

        if content.is_none() {
            return Ok(None);
        }

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
        package: &Package,
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        let content = loader.get_file_content(props_path).await?;

        if content.is_none() {
            return Ok(None);
        }

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
        packages: Vec<Package>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let java_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Java))
            .collect::<Vec<Package>>();

        info!("Found {} Java packages", java_packages.len(),);

        self.process_packages(&java_packages, loader).await
    }
}
