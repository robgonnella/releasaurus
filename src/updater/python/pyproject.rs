use log::*;

use toml_edit::{DocumentMut, value};

use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::UpdaterPackage,
};

pub struct PyProject {}

impl PyProject {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename == "pyproject.toml" {
                let mut doc = self.load_doc(&manifest.content)?;

                if let Some(project) = doc["project"].as_table_mut() {
                    if project.get("dynamic").is_some() {
                        info!(
                            "dynamic version found in pyproject.toml: skipping update"
                        );
                        continue;
                    }

                    info!(
                        "updating {} project version to {}",
                        manifest.file_path, package.next_version.semver
                    );

                    project["version"] =
                        value(package.next_version.semver.to_string());

                    file_changes.push(FileChange {
                        path: manifest.file_path.clone(),
                        content: doc.to_string(),
                        update_type: FileUpdateType::Replace,
                    });

                    continue;
                }

                if let Some(tool) = doc["tool"].as_table_mut()
                    && let Some(project) = tool["poetry"].as_table_mut()
                {
                    if project.get("dynamic").is_some() {
                        info!(
                            "dynamic version found in pyproject.toml: skipping update"
                        );
                        continue;
                    }

                    info!(
                        "updating {} tool.poetry version to {}",
                        manifest.file_path, package.next_version.semver
                    );

                    project["version"] =
                        value(package.next_version.semver.to_string());

                    file_changes.push(FileChange {
                        path: manifest.file_path.clone(),
                        content: doc.to_string(),
                        update_type: FileUpdateType::Replace,
                    });
                }
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    fn load_doc(&self, content: &str) -> Result<DocumentMut> {
        let doc = content.parse::<DocumentMut>()?;
        Ok(doc)
    }
}
