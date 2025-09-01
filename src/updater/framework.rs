use std::path::{Path, PathBuf};

use crate::updater::detection::manager::DetectionManager;
use crate::updater::detection::traits::FrameworkDetector;
use crate::updater::generic::types::{GenericMetadata, GenericPackageMetadata};
use crate::updater::generic::updater::GenericUpdater;
use crate::updater::node::detector::NodeDetector;
use crate::updater::node::types::{NodeMetadata, NodePackageMetadata};
use crate::updater::node::updater::NodeUpdater;
use crate::updater::python::detector::PythonDetector;
use crate::updater::python::types::{PythonMetadata, PythonPackageMetadata};
use crate::updater::python::updater::PythonUpdater;
use crate::updater::rust::detector::RustDetector;
use crate::updater::rust::types::{RustMetadata, RustPackageMetadata};
use crate::updater::rust::updater::CargoUpdater;
use crate::updater::traits::PackageUpdater;

/// A language/framework-agnostic package that needs version updates
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// Package name as defined in the manifest file
    pub name: String,
    /// Path to the package directory (relative to repository root)
    pub path: String,
    /// Current version of the package
    pub current_version: Option<String>,
    /// Next version to update to
    pub next_version: String,
    /// Detected framework/language for this package
    pub framework: Framework,
    /// Path to the main manifest file
    /// (e.g., Cargo.toml, package.json, setup.py)
    pub manifest_path: PathBuf,
    /// Additional metadata specific to the framework
    pub metadata: PackageMetadata,
}

/// Framework-specific metadata container
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageMetadata {
    Rust(RustPackageMetadata),
    Node(NodePackageMetadata),
    Python(PythonPackageMetadata),
    Generic(GenericPackageMetadata),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language<T> {
    pub name: String,
    pub manifest_path: PathBuf,
    pub metadata: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Supported frameworks and languages
pub enum Framework {
    /// Rust with Cargo
    Rust(Language<RustMetadata>),
    /// Node.js with npm/yarn/pnpm
    Node(Language<NodeMetadata>),
    /// Python with pip/setuptools/poetry
    Python(Language<PythonMetadata>),
    /// Generic framework with custom handling
    Generic(Language<GenericMetadata>),
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
            Framework::Rust(lang) => &lang.name,
            Framework::Node(lang) => &lang.name,
            Framework::Python(lang) => &lang.name,
            Framework::Generic(lang) => &lang.name,
        }
    }

    pub fn manifest_path(&self) -> &PathBuf {
        match &self {
            Framework::Rust(lang) => &lang.manifest_path,
            Framework::Node(lang) => &lang.manifest_path,
            Framework::Python(lang) => &lang.manifest_path,
            Framework::Generic(lang) => &lang.manifest_path,
        }
    }

    pub fn metadata(&self) -> PackageMetadata {
        match self {
            Framework::Rust(_) => PackageMetadata::Rust(RustPackageMetadata {
                is_workspace_member: false,
                is_workspace_root: false,
                local_dependencies: Vec::new(),
            }),
            Framework::Node(_) => PackageMetadata::Node(NodePackageMetadata {
                is_workspace_member: false,
                is_monorepo_root: false,
                local_dependencies: Vec::new(),
            }),
            Framework::Python(_) => {
                PackageMetadata::Python(PythonPackageMetadata {
                    local_dependencies: Vec::new(),
                    python_requires: None,
                })
            }
            Framework::Generic(_) => {
                PackageMetadata::Generic(GenericPackageMetadata {})
            }
        }
    }

    pub fn updater(&self, root_path: &Path) -> Box<dyn PackageUpdater> {
        match self {
            Framework::Rust(_) => Box::new(CargoUpdater::new(root_path)),
            Framework::Node(_) => Box::new(NodeUpdater::new(root_path)),
            Framework::Python(_) => Box::new(PythonUpdater::new(root_path)),
            Framework::Generic(_) => Box::new(GenericUpdater::new()),
        }
    }
}

impl Default for Framework {
    fn default() -> Self {
        Framework::Generic(Language {
            name: "generic".into(),
            manifest_path: PathBuf::from(""),
            metadata: GenericMetadata {
                framework_name: "unknown".into(),
                manifest_pattern: "".into(),
            },
        })
    }
}

impl Package {
    /// Create a new package with minimal information
    pub fn new(
        name: String,
        path: String,
        next_version: String,
        framework: Framework,
    ) -> Self {
        Self {
            name,
            path,
            current_version: None,
            next_version,
            framework: framework.clone(),
            manifest_path: framework.manifest_path().clone(),
            metadata: framework.metadata(),
        }
    }

    /// Set the current version of the package
    pub fn with_current_version(
        mut self,
        current_version: Option<String>,
    ) -> Self {
        self.current_version = current_version;
        self
    }

    /// Update the package metadata
    pub fn with_metadata(mut self, metadata: PackageMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Get the framework type as a string
    pub fn framework_name(&self) -> &str {
        self.framework.name()
    }

    /// Check if this package has local dependencies that need updating
    pub fn has_local_dependencies(&self) -> bool {
        match &self.metadata {
            PackageMetadata::Rust(meta) => !meta.local_dependencies.is_empty(),
            PackageMetadata::Node(meta) => !meta.local_dependencies.is_empty(),
            PackageMetadata::Python(meta) => {
                !meta.local_dependencies.is_empty()
            }
            PackageMetadata::Generic(_) => false,
        }
    }
}
