use crate::{
    forge::request::FileChange, packages::manifests::ManifestFile,
    result::Result, updater::traits::FileUpdater,
};

pub struct CompositeUpdater {
    updaters: Vec<Box<dyn FileUpdater>>,
}

impl CompositeUpdater {
    pub fn new(updaters: Vec<Box<dyn FileUpdater>>) -> Self {
        Self { updaters }
    }
}

impl FileUpdater for CompositeUpdater {
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        for updater in self.updaters.iter() {
            if let Some(change) = updater.update(manifest)? {
                return Ok(Some(change));
            }
        }

        Ok(None)
    }
}
