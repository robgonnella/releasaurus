use std::path::Path;

use crate::{result::Result, updater::framework::Package};

pub trait PackageUpdater {
    fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()>;
}
