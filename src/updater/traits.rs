use async_trait::async_trait;

use crate::{
    cli::Result, forge::request::FileChange, updater::framework::UpdaterPackage,
};

/// Common interface for updating version files in different language
/// packages.
#[async_trait]
pub trait PackageUpdater {
    /// Generate file changes to update version numbers across all relevant
    /// files for the package's language/framework.
    async fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>>;
}
