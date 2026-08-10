use quick_xml::events::{BytesText, Event};
use quick_xml::{Reader, Writer as XmlWriter};

use crate::forge::request::{FileChange, FileUpdateType};
use crate::packages::manifests::ManifestFile;
use crate::result::Result;
use crate::updater::traits::FileUpdater;

/// Handles Maven pom.xml file parsing and version updates for Java packages.
pub struct Maven {}

impl Default for Maven {
    fn default() -> Self {
        Maven::new()
    }
}

impl Maven {
    /// Create Maven handler for pom.xml version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update a single pom.xml file
    fn update_pom_file(
        &self,
        manifest: &ManifestFile,
    ) -> Result<Option<FileChange>> {
        let Some(owner) = manifest.owner.as_ref() else {
            return Ok(None);
        };

        log::info!(
            "Updating Maven project: {}",
            manifest.path.to_string_lossy()
        );

        let bytes = manifest.content.as_bytes();

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
                        let new_version = owner.tag.semver.to_string();
                        log::info!(
                            "Updating Maven version to: {}",
                            new_version
                        );
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
            path: manifest.path.to_string_lossy().to_string(),
            content,
            update_type: FileUpdateType::Replace,
        }))
    }
}

impl FileUpdater for Maven {
    /// Update version fields in pom.xml files for all Java packages.
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename == "pom.xml" {
            return self.update_pom_file(manifest);
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        config::release_type::ReleaseType, forge::request::Tag,
        packages::manifests::ManifestPackage,
    };

    use super::*;

    #[test]
    fn updates_project_version() {
        let maven = Maven::new();
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <version>1.0.0</version>
</project>"#;

        let manifest = ManifestFile {
            path: Path::new("pom.xml").to_path_buf(),
            basename: "pom.xml".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".to_string(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = maven.update_pom_file(&manifest).unwrap();

        let updated = result.unwrap().content;
        assert!(updated.contains("<version>2.0.0</version>"));
    }

    #[test]
    fn preserves_xml_structure() {
        let maven = Maven::new();
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>my-app</artifactId>
    <version>1.0.0</version>
    <dependencies>
        <dependency>
            <groupId>junit</groupId>
            <artifactId>junit</artifactId>
            <version>4.12</version>
        </dependency>
    </dependencies>
</project>"#;

        let manifest = ManifestFile {
            path: Path::new("pom.xml").to_path_buf(),
            basename: "pom.xml".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".to_string(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = maven.update_pom_file(&manifest).unwrap();
        let updated = result.unwrap().content;
        assert!(updated.contains("<groupId>com.example</groupId>"));
        assert!(updated.contains("<artifactId>my-app</artifactId>"));
        assert!(updated.contains("<version>2.0.0</version>"));
        assert!(updated.contains("<groupId>junit</groupId>"));
        assert!(updated.contains("<version>4.12</version>"));
    }

    #[test]
    fn only_updates_project_level_version() {
        let maven = Maven::new();
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <version>1.0.0</version>
    <dependencies>
        <dependency>
            <version>4.12</version>
        </dependency>
    </dependencies>
</project>"#;

        let manifest = ManifestFile {
            path: Path::new("pom.xml").to_path_buf(),
            basename: "pom.xml".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".to_string(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v3.0.0".into(),
                    semver: semver::Version::new(3, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = maven.update_pom_file(&manifest).unwrap();
        let updated = result.unwrap().content;
        assert!(updated.contains("<version>3.0.0</version>"));
        assert!(updated.contains("<version>4.12</version>"));
    }

    #[test]
    fn handles_multiline_xml() {
        let maven = Maven::new();
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>test-app</artifactId>
    <version>1.0.0</version>
    <packaging>jar</packaging>
</project>"#;

        let manifest = ManifestFile {
            path: Path::new("pom.xml").to_path_buf(),
            basename: "pom.xml".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".to_string(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.5.0".into(),
                    semver: semver::Version::new(2, 5, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = maven.update_pom_file(&manifest).unwrap();
        let updated = result.unwrap().content;
        assert!(updated.contains("<version>2.5.0</version>"));
        assert!(updated.contains("<modelVersion>4.0.0</modelVersion>"));
        assert!(updated.contains("<packaging>jar</packaging>"));
    }

    #[test]
    fn process_package_returns_none_when_no_pom_files() {
        let maven = Maven::new();

        let manifest = ManifestFile {
            path: Path::new("build.gradle").to_path_buf(),
            basename: "build.gradle".to_string(),
            content: "version = \"1.0.0\"".to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".to_string(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = maven.update(&manifest).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn handles_parent_pom_structure() {
        let maven = Maven::new();
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <parent>
        <groupId>com.example</groupId>
        <artifactId>parent</artifactId>
        <version>5.0.0</version>
    </parent>
    <version>1.0.0</version>
</project>"#;

        let manifest = ManifestFile {
            path: Path::new("pom.xml").to_path_buf(),
            basename: "pom.xml".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".to_string(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v3.0.0".into(),
                    semver: semver::Version::new(3, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = maven.update_pom_file(&manifest).unwrap();
        let updated = result.unwrap().content;
        assert!(updated.contains("<version>3.0.0</version>"));
        assert!(updated.contains("<version>5.0.0</version>"));
    }
}
