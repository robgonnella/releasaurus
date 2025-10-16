use log::*;
use regex::Regex;
use std::sync::LazyLock;

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
    updater::framework::UpdaterPackage,
};

static VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^(\s*version\s*=\s*[\"'][\w\.\-\+]+[\"'])"#).unwrap()
});

pub struct SetupPy {}

impl SetupPy {
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
            let file_path = package.get_file_path("setup.py");

            let content = self.load_doc(&file_path, loader).await?;

            if content.is_none() {
                continue;
            }

            let mut content = content.unwrap();

            info!("found setup.py for package: {}", package.path);

            let updated_version =
                format!("version = {}", package.next_version.semver);

            content =
                VERSION_REGEX.replace(&content, updated_version).to_string();

            file_changes.push(FileChange {
                path: file_path,
                content,
                update_type: FileUpdateType::Replace,
            });
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
    ) -> Result<Option<String>> {
        let content = loader.get_file_content(file_path).await?;
        if content.is_none() {
            return Ok(None);
        }
        let content = content.unwrap();
        Ok(Some(content))
    }
}
