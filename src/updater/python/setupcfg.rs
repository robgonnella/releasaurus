use regex::Regex;
use std::sync::LazyLock;

use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::UpdaterPackage,
};

static VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^(\s*version\s*=\s*)([\"']?)([\w\.\-\+]+)([\"']?)"#)
        .unwrap()
});

pub struct SetupCfg {}

impl SetupCfg {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename == "setup.cfg" {
                let content = VERSION_REGEX
                    .replace(&manifest.content, |caps: &regex::Captures| {
                        format!(
                            "{}{}{}{}",
                            &caps[1],
                            &caps[2],
                            package.next_version.semver,
                            &caps[4]
                        )
                    })
                    .to_string();

                file_changes.push(FileChange {
                    path: manifest.file_path.clone(),
                    content,
                    update_type: FileUpdateType::Replace,
                });
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}
