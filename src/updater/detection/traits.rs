use std::path::Path;

use crate::{result::Result, updater::detection::types::FrameworkDetection};

pub trait FrameworkDetector {
    fn name(&self) -> &str;
    fn detect(&self, path: &Path) -> Result<FrameworkDetection>;
}
