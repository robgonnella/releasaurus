use log::*;

use toml_edit::{DocumentMut, value};

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
    updater::framework::UpdaterPackage,
};

pub struct PyProject {}

impl PyProject {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn process_packages(
        &self,
        packages: &[UpdaterPackage],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for package in packages {
            let file_path = package.get_file_path("pyproject.toml");

            let doc = self.load_doc(&file_path, loader).await?;

            if doc.is_none() {
                continue;
            }

            info!("found pyproject.toml for package: {}", package.path);

            let mut doc = doc.unwrap();

            if let Some(project) = doc["project"].as_table_mut() {
                if project.get("dynamic").is_some() {
                    info!(
                        "dynamic version found in pyproject.toml: skipping update"
                    );
                    continue;
                }

                info!(
                    "updating {} project version to {}",
                    file_path, package.next_version.semver
                );

                project["version"] =
                    value(package.next_version.semver.to_string());

                file_changes.push(FileChange {
                    path: file_path,
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
                    file_path, package.next_version.semver
                );

                project["version"] =
                    value(package.next_version.semver.to_string());

                file_changes.push(FileChange {
                    path: file_path,
                    content: doc.to_string(),
                    update_type: FileUpdateType::Replace,
                });
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    async fn load_doc(
        &self,
        file_path: &str,
        loader: &dyn FileLoader,
    ) -> Result<Option<DocumentMut>> {
        let content = loader.get_file_content(file_path).await?;
        if content.is_none() {
            return Ok(None);
        }
        let content = content.unwrap();
        let doc = content.parse::<DocumentMut>()?;
        Ok(Some(doc))
    }
}
