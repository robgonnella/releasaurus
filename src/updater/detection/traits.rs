use std::path::Path;

use color_eyre::eyre::Result;

use crate::updater::detection::types::FrameworkDetection;

pub trait FrameworkDetector {
    fn name(&self) -> &str;
    fn detect(&self, path: &Path) -> Result<FrameworkDetection>;
}
