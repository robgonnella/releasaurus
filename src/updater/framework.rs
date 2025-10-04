//! Framework and package management for multi-language support.

use crate::analyzer::release::Tag;
use crate::config::ReleaseType;
use crate::updater::generic::updater::GenericUpdater;
use crate::updater::java::updater::JavaUpdater;
use crate::updater::node::updater::NodeUpdater;
use crate::updater::php::updater::PhpUpdater;
use crate::updater::python::updater::PythonUpdater;
use crate::updater::ruby::updater::RubyUpdater;
use crate::updater::rust::updater::RustUpdater;
use crate::updater::traits::PackageUpdater;

/// Programming language and package manager detection for determining which
/// version files to update.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Framework {
    /// Rust with Cargo
    Rust,
    /// Node.js with npm/yarn/pnpm
    Node,
    /// Python with pip/setuptools/poetry
    Python,
    /// PHP with Composer
    Php,
    /// Java with Maven/Gradle
    Java,
    /// Ruby with Bundler/Gems
    Ruby,
    #[default]
    /// Generic framework with custom handling
    Generic,
}

impl From<ReleaseType> for Framework {
    fn from(value: ReleaseType) -> Self {
        match value {
            ReleaseType::Generic => Framework::Generic,
            ReleaseType::Java => Framework::Java,
            ReleaseType::Node => Framework::Node,
            ReleaseType::Php => Framework::Php,
            ReleaseType::Python => Framework::Python,
            ReleaseType::Ruby => Framework::Ruby,
            ReleaseType::Rust => Framework::Rust,
        }
    }
}

impl Framework {
    /// Get language-specific updater implementation for this framework.
    pub fn updater(&self) -> Box<dyn PackageUpdater> {
        match self {
            Framework::Rust => Box::new(RustUpdater::new()),
            Framework::Node => Box::new(NodeUpdater::new()),
            Framework::Python => Box::new(PythonUpdater::new()),
            Framework::Php => Box::new(PhpUpdater::new()),
            Framework::Java => Box::new(JavaUpdater::new()),
            Framework::Ruby => Box::new(RubyUpdater::new()),
            Framework::Generic => Box::new(GenericUpdater::new()),
        }
    }
}

/// Package information with next version and framework details for version
/// file updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// Package name derived from manifest or directory.
    pub name: String,
    /// Path to package directory relative to repository root.
    pub path: String,
    /// Next version to update to based on commit analysis.
    pub next_version: Tag,
    /// Language/framework for selecting appropriate updater.
    pub framework: Framework,
}

impl Package {
    /// Create package instance with name, path, version, and framework
    /// detection.
    pub fn new(
        name: String,
        path: String,
        next_version: Tag,
        framework: Framework,
    ) -> Self {
        Self {
            name,
            path,
            next_version,
            framework: framework.clone(),
        }
    }
}
