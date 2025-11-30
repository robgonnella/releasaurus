//! Framework and package management for multi-language support.
use log::*;
use std::fmt::Display;

use crate::Result;
use crate::analyzer::release::Tag;
use crate::command::types::ReleasablePackage;
use crate::config::manifest::ManifestFile;
use crate::config::release_type::ReleaseType;
use crate::forge::request::FileChange;
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
    #[default]
    /// Generic framework with custom handling
    Generic,
    /// Java with Maven/Gradle
    Java,
    /// Node.js with npm/yarn/pnpm
    Node,
    /// PHP with Composer
    Php,
    /// Python with pip/setuptools/poetry
    Python,
    /// Ruby with Bundler/Gems
    Ruby,
    /// Rust with Cargo
    Rust,
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
    pub async fn update_package(
        package: &ReleasablePackage,
        all_packages: &[ReleasablePackage],
    ) -> Result<Vec<FileChange>> {
        let mut file_changes = vec![];

        let package = UpdaterPackage::from_releasable_package(package);

        let all_packages = all_packages
            .iter()
            .map(UpdaterPackage::from_releasable_package)
            .collect::<Vec<UpdaterPackage>>();

        let mut workspace_packages = vec![];

        // gather other packages related to target package that may be in
        // same workspace
        for pkg in all_packages {
            if pkg.package_name != package.package_name
                && pkg.workspace_root == package.workspace_root
                && pkg.framework == package.framework
            {
                workspace_packages.push(pkg.clone());
            }
        }

        info!(
            "Package: {}: Found {} other packages for workspace root: {}, framework: {}",
            package.package_name,
            workspace_packages.len(),
            package.workspace_root,
            package.framework
        );

        let updater = package.framework.updater();

        if let Some(changes) = updater.update(&package, workspace_packages)? {
            file_changes.extend(changes);
        }

        Ok(file_changes)
    }

    /// Get language-specific updater implementation for this framework.
    fn updater(&self) -> Box<dyn PackageUpdater> {
        match self {
            Framework::Generic => Box::new(GenericUpdater::new()),
            Framework::Java => Box::new(JavaUpdater::new()),
            Framework::Node => Box::new(NodeUpdater::new()),
            Framework::Php => Box::new(PhpUpdater::new()),
            Framework::Python => Box::new(PythonUpdater::new()),
            Framework::Ruby => Box::new(RubyUpdater::new()),
            Framework::Rust => Box::new(RustUpdater::new()),
        }
    }
}

/// Package information with next version and framework details for version
/// file updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdaterPackage {
    /// Package name derived from manifest or directory.
    pub package_name: String,
    /// Path to the workspace root directory for this package relative to the repository root
    pub workspace_root: String,
    /// List of manifest files to update
    pub manifest_files: Vec<ManifestFile>,
    /// Next version to update to based on commit analysis.
    pub next_version: Tag,
    /// Language/framework for selecting appropriate updater.
    pub framework: Framework,
}

impl UpdaterPackage {
    fn from_releasable_package(pkg: &ReleasablePackage) -> Self {
        let framework = Framework::from(pkg.release_type.clone());

        let tag = pkg.release.tag.clone().unwrap_or_default();

        UpdaterPackage {
            package_name: pkg.name.clone(),
            workspace_root: pkg.workspace_root.clone(),
            framework,
            manifest_files: pkg.manifest_files.clone().unwrap_or_default(),
            next_version: tag,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::analyzer::release::Release;

    use super::*;
    use semver::Version as SemVer;

    // ===== Test Helpers =====

    /// Creates a minimal releasable package for testing
    fn releasable_package(
        name: &str,
        release_type: ReleaseType,
    ) -> ReleasablePackage {
        ReleasablePackage {
            name: name.to_string(),
            path: ".".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: None,
            additional_manifest_files: None,
            release_type,
            release: Release {
                tag: Some(Tag {
                    sha: "test-sha".to_string(),
                    name: "v1.0.0".to_string(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                    timestamp: Some(100),
                }),
                link: String::new(),
                sha: "test-sha".to_string(),
                commits: vec![],
                include_author: false,
                notes: String::new(),
                timestamp: 0,
            },
        }
    }

    // ===== Framework Conversion Tests =====

    #[test]
    fn converts_release_type_to_framework() {
        assert_eq!(Framework::from(ReleaseType::Generic), Framework::Generic);
        assert_eq!(Framework::from(ReleaseType::Java), Framework::Java);
        assert_eq!(Framework::from(ReleaseType::Node), Framework::Node);
        assert_eq!(Framework::from(ReleaseType::Php), Framework::Php);
        assert_eq!(Framework::from(ReleaseType::Python), Framework::Python);
        assert_eq!(Framework::from(ReleaseType::Ruby), Framework::Ruby);
        assert_eq!(Framework::from(ReleaseType::Rust), Framework::Rust);
    }

    #[test]
    fn displays_framework_as_lowercase() {
        assert_eq!(Framework::Generic.to_string(), "generic");
        assert_eq!(Framework::Java.to_string(), "java");
        assert_eq!(Framework::Node.to_string(), "node");
        assert_eq!(Framework::Php.to_string(), "php");
        assert_eq!(Framework::Python.to_string(), "python");
        assert_eq!(Framework::Ruby.to_string(), "ruby");
        assert_eq!(Framework::Rust.to_string(), "rust");
    }

    // ===== UpdaterPackage Tests =====

    #[test]
    fn converts_releasable_to_updater_package() {
        let releasable = releasable_package("my-pkg", ReleaseType::Node);

        let updater = UpdaterPackage::from_releasable_package(&releasable);

        assert_eq!(updater.package_name, "my-pkg");
        assert_eq!(updater.workspace_root, ".");
        assert_eq!(updater.framework, Framework::Node);
        assert_eq!(updater.next_version.name, "v1.0.0");
    }

    #[test]
    fn handles_missing_manifest_files() {
        let releasable = releasable_package("pkg", ReleaseType::Generic);

        let updater = UpdaterPackage::from_releasable_package(&releasable);

        assert_eq!(updater.manifest_files.len(), 0);
    }

    #[tokio::test]
    async fn returns_empty_changes_for_generic_framework() {
        let pkg = releasable_package("pkg", ReleaseType::Generic);

        let changes = Framework::update_package(&pkg, &[]).await.unwrap();

        assert_eq!(changes.len(), 0);
    }
}
