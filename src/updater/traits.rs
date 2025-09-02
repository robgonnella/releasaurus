use color_eyre::eyre::Result;
use std::path::Path;

use crate::updater::framework::Package;

pub trait PackageUpdater {
    fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()>;
}
