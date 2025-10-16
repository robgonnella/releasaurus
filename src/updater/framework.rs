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
    /// Path to package directory relative to workspace_root path
    pub path: String,
    /// Path to the workspace root directory for this package relative to the repository root
    pub workspace_root: String,
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
            workspace_root: pkg.workspace_root.clone(),
            next_version,
            framework,
        })
    }

    /// Construct a normalized file path by joining workspace_root, path, and filename.
    /// Strips leading "./" from the result for consistency.
    pub fn get_file_path(&self, filename: &str) -> String {
        use std::path::Path;

        let full_path = Path::new(&self.workspace_root)
            .join(&self.path)
            .join(filename);

        let path_str = full_path.display().to_string();

        // Strip leading "./" for normalized paths
        path_str.strip_prefix("./").unwrap_or(&path_str).to_string()
    }

    /// Construct a normalized workspace-level file path.
    /// Strips leading "./" from the result for consistency.
    pub fn get_workspace_file_path(&self, filename: &str) -> String {
        use std::path::Path;

        let full_path = Path::new(&self.workspace_root).join(filename);
        let path_str = full_path.display().to_string();

        // Strip leading "./" for normalized paths
        path_str.strip_prefix("./").unwrap_or(&path_str).to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version as SemVer;

    #[test]
    fn test_updater_package_get_file_path() {
        let package = UpdaterPackage {
            name: "test-package".to_string(),
            path: "packages/test".to_string(),
            workspace_root: ".".to_string(),
            next_version: Tag {
                sha: "abc123".to_string(),
                name: "v1.0.0".to_string(),
                semver: SemVer::parse("1.0.0").unwrap(),
            },
            framework: Framework::Rust,
        };

        assert_eq!(
            package.get_file_path("Cargo.toml"),
            "packages/test/Cargo.toml"
        );
        assert_eq!(
            package.get_file_path("package.json"),
            "packages/test/package.json"
        );
    }

    #[test]
    fn test_updater_package_get_file_path_with_empty_workspace() {
        let package = UpdaterPackage {
            name: "test-package".to_string(),
            path: "packages/test".to_string(),
            workspace_root: "".to_string(),
            next_version: Tag {
                sha: "abc123".to_string(),
                name: "v1.0.0".to_string(),
                semver: SemVer::parse("1.0.0").unwrap(),
            },
            framework: Framework::Rust,
        };

        assert_eq!(
            package.get_file_path("Cargo.toml"),
            "packages/test/Cargo.toml"
        );
    }

    #[test]
    fn test_updater_package_get_file_path_with_custom_workspace() {
        let package = UpdaterPackage {
            name: "test-package".to_string(),
            path: "api".to_string(),
            workspace_root: "rust-workspace".to_string(),
            next_version: Tag {
                sha: "abc123".to_string(),
                name: "v1.0.0".to_string(),
                semver: SemVer::parse("1.0.0").unwrap(),
            },
            framework: Framework::Rust,
        };

        assert_eq!(
            package.get_file_path("Cargo.toml"),
            "rust-workspace/api/Cargo.toml"
        );
    }

    #[test]
    fn test_updater_package_get_workspace_file_path() {
        let package = UpdaterPackage {
            name: "test-package".to_string(),
            path: "packages/test".to_string(),
            workspace_root: ".".to_string(),
            next_version: Tag {
                sha: "abc123".to_string(),
                name: "v1.0.0".to_string(),
                semver: SemVer::parse("1.0.0").unwrap(),
            },
            framework: Framework::Rust,
        };

        assert_eq!(package.get_workspace_file_path("Cargo.lock"), "Cargo.lock");
        assert_eq!(
            package.get_workspace_file_path("package-lock.json"),
            "package-lock.json"
        );
    }

    #[test]
    fn test_updater_package_get_workspace_file_path_with_custom_workspace() {
        let package = UpdaterPackage {
            name: "test-package".to_string(),
            path: "api".to_string(),
            workspace_root: "rust-workspace".to_string(),
            next_version: Tag {
                sha: "abc123".to_string(),
                name: "v1.0.0".to_string(),
                semver: SemVer::parse("1.0.0").unwrap(),
            },
            framework: Framework::Rust,
        };

        assert_eq!(
            package.get_workspace_file_path("Cargo.lock"),
            "rust-workspace/Cargo.lock"
        );
    }
}
