use log::*;
use quick_xml::events::{BytesText, Event};
use quick_xml::{Reader, Writer as XmlWriter};

use crate::updater::framework::ManifestFile;
use crate::{
    forge::request::{FileChange, FileUpdateType},
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
    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename == "pom.xml"
                && let Some(change) =
                    self.update_pom_file(manifest, package).await?
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
        manifest: &ManifestFile,
        package: &UpdaterPackage,
    ) -> Result<Option<FileChange>> {
        info!("Updating Maven project: {}", manifest.file_path);

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
            path: manifest.file_path.clone(),
            content,
            update_type: FileUpdateType::Replace,
        }))
    }
}
