use std::path::Path;

use crate::{result::Result, updater::detection::types::FrameworkDetection};

/// Common interface for detecting programming language frameworks.
pub trait FrameworkDetector {
    /// Get detector name.
    fn name(&self) -> &str;
    /// Detect framework in the given path.
    fn detect(&self, path: &Path) -> Result<FrameworkDetection>;
}
