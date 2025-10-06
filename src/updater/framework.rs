//! Framework and package management for multi-language support.
use color_eyre::eyre::eyre;
use std::fmt::Display;

use crate::analyzer::release::Tag;
use crate::config::ReleaseType;
use crate::result::{ReleasablePackage, Result};
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

impl Display for Framework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Framework::Generic => f.write_str("generic"),
            Framework::Java => f.write_str("java"),
            Framework::Node => f.write_str("node"),
            Framework::Php => f.write_str("php"),
            Framework::Python => f.write_str("python"),
            Framework::Ruby => f.write_str("ruby"),
            Framework::Rust => f.write_str("rust"),
        }
    }
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
pub struct UpdaterPackage {
    /// Package name derived from manifest or directory.
    pub name: String,
    /// Path to package directory relative to repository root.
    pub path: String,
    /// Next version to update to based on commit analysis.
    pub next_version: Tag,
    /// Language/framework for selecting appropriate updater.
    pub framework: Framework,
}

impl UpdaterPackage {
    pub fn from_manifest_package(pkg: &ReleasablePackage) -> Result<Self> {
        if pkg.release.tag.is_none() {
            return Err(eyre!("failed to find tag for next release"));
        }

        let next_version = pkg.release.tag.clone().unwrap();
        let framework = Framework::from(pkg.release_type.clone());

        Ok(Self {
            name: pkg.name.clone(),
            path: pkg.path.clone(),
            next_version,
            framework,
        })
    }
}

pub fn updater_packages_from_manifest(
    manifest: &[ReleasablePackage],
) -> Result<Vec<UpdaterPackage>> {
    let mut packages = vec![];

    for pkg in manifest.iter() {
        packages.push(UpdaterPackage::from_manifest_package(pkg)?);
    }

    Ok(packages)
}
