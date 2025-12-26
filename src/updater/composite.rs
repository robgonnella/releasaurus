use crate::{
    Result,
    forge::request::FileChange,
    updater::{manager::UpdaterPackage, traits::PackageUpdater},
};

pub struct CompositeUpdater {
    updaters: Vec<Box<dyn PackageUpdater>>,
}

impl CompositeUpdater {
    pub fn new(updaters: Vec<Box<dyn PackageUpdater>>) -> Self {
        Self { updaters }
    }
}

impl PackageUpdater for CompositeUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        for updater in self.updaters.iter() {
            if let Some(changes) =
                updater.update(package, workspace_packages)?
            {
                file_changes.extend(changes);
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}
