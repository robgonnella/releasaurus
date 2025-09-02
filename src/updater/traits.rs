use color_eyre::eyre::Result;

use crate::updater::framework::Package;

pub trait PackageUpdater {
    fn update(&self, packages: Vec<Package>) -> Result<()>;
}
