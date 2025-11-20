use async_trait::async_trait;

use crate::{
    forge::request::FileChange,
    result::Result,
    updater::{
        framework::UpdaterPackage, php::composer_json::ComposerJson,
        traits::PackageUpdater,
    },
};

/// PHP package updater for Composer projects.
pub struct PhpUpdater {
    composer_json: ComposerJson,
}

impl PhpUpdater {
    /// Create PHP updater for Composer composer.json files.
    pub fn new() -> Self {
        Self {
            composer_json: ComposerJson::new(),
        }
    }
}

#[async_trait]
impl PackageUpdater for PhpUpdater {
    async fn update(
        &self,
        package: &UpdaterPackage,
        // workspaces not supported for php projects
        _workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        self.composer_json.process_package(package).await
    }
}
