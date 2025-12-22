use log::*;
use quick_xml::events::{BytesText, Event};
use quick_xml::{Reader, Writer as XmlWriter};

use crate::updater::manager::ManifestFile;
use crate::{
    Result,
    forge::request::{FileChange, FileUpdateType},
    updater::manager::UpdaterPackage,
};

/// Handles Maven pom.xml file parsing and version updates for Java packages.
pub struct Maven {}

impl Maven {
    /// Create Maven handler for pom.xml version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version fields in pom.xml files for all Java packages.
    pub fn process_package(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.basename == "pom.xml"
                && let Some(change) = self.update_pom_file(manifest, package)?
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
    fn update_pom_file(
        &self,
        manifest: &ManifestFile,
        package: &UpdaterPackage,
    ) -> Result<Option<FileChange>> {
        info!("Updating Maven project: {}", manifest.path);

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
            path: manifest.path.clone(),
            content,
            update_type: FileUpdateType::Replace,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::release::Tag, config::release_type::ReleaseType,
        updater::manager::UpdaterPackage,
    };

    #[test]
    fn updates_project_version() {
        let maven = Maven::new();
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <version>1.0.0</version>
</project>"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "pom.xml".to_string(),
            basename: "pom.xml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Java,
        };

        let result = maven.update_pom_file(&manifest, &package).unwrap();

        assert!(result.is_some());
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
            is_workspace: false,
            path: "pom.xml".to_string(),
            basename: "pom.xml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Java,
        };

        let result = maven.update_pom_file(&manifest, &package).unwrap();

        assert!(result.is_some());
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
            is_workspace: false,
            path: "pom.xml".to_string(),
            basename: "pom.xml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::parse("3.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Java,
        };

        let result = maven.update_pom_file(&manifest, &package).unwrap();

        assert!(result.is_some());
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
            is_workspace: false,
            path: "pom.xml".to_string(),
            basename: "pom.xml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.5.0".into(),
                semver: semver::Version::parse("2.5.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Java,
        };

        let result = maven.update_pom_file(&manifest, &package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap().content;
        assert!(updated.contains("<version>2.5.0</version>"));
        assert!(updated.contains("<modelVersion>4.0.0</modelVersion>"));
        assert!(updated.contains("<packaging>jar</packaging>"));
    }

    #[test]
    fn process_package_handles_multiple_pom_files() {
        let maven = Maven::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            path: "module1/pom.xml".to_string(),
            basename: "pom.xml".to_string(),
            content: r#"<?xml version="1.0"?><project><version>1.0.0</version></project>"#
                .to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            path: "module2/pom.xml".to_string(),
            basename: "pom.xml".to_string(),
            content: r#"<?xml version="1.0"?><project><version>1.0.0</version></project>"#
                .to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Java,
        };

        let result = maven.process_package(&package).unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[test]
    fn process_package_returns_none_when_no_pom_files() {
        let maven = Maven::new();
        let manifest = ManifestFile {
            is_workspace: false,
            path: "build.gradle".to_string(),
            basename: "build.gradle".to_string(),
            content: "version = \"1.0.0\"".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Java,
        };

        let result = maven.process_package(&package).unwrap();

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
            is_workspace: false,
            path: "pom.xml".to_string(),
            basename: "pom.xml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::parse("3.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Java,
        };

        let result = maven.update_pom_file(&manifest, &package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap().content;
        assert!(updated.contains("<version>3.0.0</version>"));
        assert!(updated.contains("<version>5.0.0</version>"));
    }
}
