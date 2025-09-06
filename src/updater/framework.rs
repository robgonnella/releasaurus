use std::path::PathBuf;

use crate::analyzer::types::Version;
use crate::updater::detection::manager::DetectionManager;
use crate::updater::detection::traits::FrameworkDetector;
use crate::updater::generic::updater::GenericUpdater;
use crate::updater::node::detector::NodeDetector;
use crate::updater::node::updater::NodeUpdater;
use crate::updater::python::detector::PythonDetector;
use crate::updater::python::updater::PythonUpdater;
use crate::updater::rust::detector::RustDetector;
use crate::updater::rust::updater::CargoUpdater;
use crate::updater::traits::PackageUpdater;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
/// Supported frameworks and languages
pub enum Framework {
    /// Rust with Cargo
    Rust,
    /// Node.js with npm/yarn/pnpm
    Node,
    /// Python with pip/setuptools/poetry
    Python,
    #[default]
    /// Generic framework with custom handling
    Generic,
}

impl Framework {
    pub fn detection_manager(root_path: PathBuf) -> DetectionManager {
        let detectors: Vec<Box<dyn FrameworkDetector>> = vec![
            Box::new(RustDetector::new()),
            Box::new(PythonDetector::new()),
            Box::new(NodeDetector::new()),
        ];

        DetectionManager::new(root_path, detectors)
    }

    pub fn name(&self) -> &str {
        match self {
            Framework::Rust => "rust",
            Framework::Node => "node",
            Framework::Python => "python",
            Framework::Generic => "unknown",
        }
    }

    pub fn updater(&self) -> Box<dyn PackageUpdater> {
        match self {
            Framework::Rust => Box::new(CargoUpdater::new()),
            Framework::Node => Box::new(NodeUpdater::new()),
            Framework::Python => Box::new(PythonUpdater::new()),
            Framework::Generic => Box::new(GenericUpdater::new()),
        }
    }
}

/// A language/framework-agnostic package that needs version updates
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// Package name as defined in the manifest file
    pub name: String,
    /// Path to the package directory (relative to repository root)
    pub path: String,
    /// Next version to update to
    pub next_version: Version,
    /// Detected framework/language for this package
    pub framework: Framework,
}

impl Package {
    /// Create a new package with minimal information
    pub fn new(
        name: String,
        path: String,
        next_version: Version,
        framework: Framework,
    ) -> Self {
        Self {
            name,
            path,
            next_version,
            framework: framework.clone(),
        }
    }

    /// Get the framework type as a string
    pub fn framework_name(&self) -> &str {
        self.framework.name()
    }
}
