//! Framework and package management for multi-language support.
use log::*;
use std::fmt::Display;
use std::path::Path;

use crate::analyzer::release::Tag;
use crate::config::ReleaseType;
use crate::forge::request::FileChange;
use crate::forge::traits::Forge;
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
        forge: &dyn Forge,
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

        // populate package manifests with content
        let mut package = package.clone();
        let mut package_manifests = vec![];

        for manifest in package.manifest_files.iter_mut() {
            if let Some(content) =
                forge.get_file_content(&manifest.file_path).await?
            {
                manifest.content = content;
                package_manifests.push(manifest.clone());
            }
        }

        package.manifest_files = package_manifests;

        // populate other workspace package manifests with content
        for pkg in workspace_packages.iter_mut() {
            let mut manifest_files = vec![];

            for manifest in pkg.manifest_files.iter_mut() {
                if let Some(content) =
                    forge.get_file_content(&manifest.file_path).await?
                {
                    manifest.content = content;
                    manifest_files.push(manifest.clone());
                }
            }

            pkg.manifest_files = manifest_files
        }

        let updater = package.framework.updater();

        if let Some(changes) =
            updater.update(&package, workspace_packages).await?
        {
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

    pub fn manifest_files(
        &self,
        package: &ReleasablePackage,
    ) -> Vec<ManifestFile> {
        let gen_package_path = |file: &str| -> String {
            Path::new(&package.workspace_root)
                .join(&package.path)
                .join(file)
                .display()
                .to_string()
                .replace("./", "")
        };

        let gen_workspace_path = |file: &str| -> String {
            Path::new(&package.workspace_root)
                .join(file)
                .display()
                .to_string()
                .replace("./", "")
        };

        match self {
            Framework::Generic => vec![],
            Framework::Java => {
                vec![
                    ManifestFile {
                        content: "".to_string(),
                        file_basename: "build.gradle".into(),
                        file_path: gen_package_path("build.gradle"),
                        is_workspace: false,
                    },
                    ManifestFile {
                        content: "".to_string(),
                        file_basename: "build.gradle.kts".into(),
                        file_path: gen_package_path("build.gradle.kts"),
                        is_workspace: false,
                    },
                    ManifestFile {
                        content: "".to_string(),
                        file_basename: "gradle.properties".into(),
                        file_path: gen_package_path("gradle.properties"),
                        is_workspace: false,
                    },
                    ManifestFile {
                        content: "".to_string(),
                        file_basename: "pom.xml".into(),
                        file_path: gen_package_path("pom.xml"),
                        is_workspace: false,
                    },
                ]
            }
            Framework::Node => {
                let pkg_lock_pkg_path = gen_package_path("package.json");
                let pkg_lock_workspace_path =
                    gen_workspace_path("package-lock.json");

                if pkg_lock_pkg_path == pkg_lock_workspace_path {
                    // package is not part of a workspace with other packages
                    vec![
                        ManifestFile {
                            content: "".to_string(),
                            file_basename: "package.json".into(),
                            file_path: pkg_lock_pkg_path,
                            is_workspace: false,
                        },
                        ManifestFile {
                            content: "".to_string(),
                            file_basename: "package-lock.json".into(),
                            file_path: gen_package_path("package-lock.json"),
                            is_workspace: false,
                        },
                        ManifestFile {
                            content: "".to_string(),
                            file_basename: "yarn.lock".into(),
                            file_path: gen_package_path("yarn.lock"),
                            is_workspace: false,
                        },
                    ]
                } else {
                    // package is part of workspace with other packages so
                    // include workspace root manifest files
                    vec![
                        ManifestFile {
                            content: "".to_string(),
                            file_basename: "package.json".into(),
                            file_path: gen_package_path("package.json"),
                            is_workspace: false,
                        },
                        ManifestFile {
                            content: "".to_string(),
                            file_basename: "package-lock.json".into(),
                            file_path: gen_package_path("package-lock.json"),
                            is_workspace: false,
                        },
                        ManifestFile {
                            content: "".to_string(),
                            file_basename: "yarn.lock".into(),
                            file_path: gen_package_path("yarn.lock"),
                            is_workspace: false,
                        },
                        ManifestFile {
                            content: "".to_string(),
                            file_basename: "package-lock.json".into(),
                            file_path: gen_workspace_path("package-lock.json"),
                            is_workspace: true,
                        },
                        ManifestFile {
                            content: "".to_string(),
                            file_basename: "yarn.lock".into(),
                            file_path: gen_workspace_path("yarn.lock"),
                            is_workspace: true,
                        },
                    ]
                }
            }
            Framework::Php => {
                vec![ManifestFile {
                    content: "".to_string(),
                    file_basename: "composer.json".into(),
                    file_path: gen_package_path("composer.json"),
                    is_workspace: false,
                }]
            }
            Framework::Python => {
                vec![
                    ManifestFile {
                        content: "".to_string(),
                        file_basename: "pyproject.toml".into(),
                        file_path: gen_package_path("pyproject.toml"),
                        is_workspace: false,
                    },
                    ManifestFile {
                        content: "".to_string(),
                        file_basename: "setup.cfg".into(),
                        file_path: gen_package_path("setup.cfg"),
                        is_workspace: false,
                    },
                    ManifestFile {
                        content: "".to_string(),
                        file_basename: "setup.py".into(),
                        file_path: gen_package_path("setup.py"),
                        is_workspace: false,
                    },
                ]
            }
            Framework::Ruby => {
                let pkg_gemspec = format!("{}.gemspec", package.name);
                let lib_pkg_version =
                    format!("lib/{}/version.rb", package.name);
                vec![
                    ManifestFile {
                        content: "".to_string(),
                        file_basename: pkg_gemspec.clone(),
                        file_path: gen_package_path(&pkg_gemspec),
                        is_workspace: false,
                    },
                    ManifestFile {
                        content: "".to_string(),
                        file_basename: "version.rb".into(),
                        file_path: gen_package_path("version.rb"),
                        is_workspace: false,
                    },
                    ManifestFile {
                        content: "".to_string(),
                        file_basename: "lib/version.rb".into(),
                        file_path: gen_package_path("lib/version.rb"),
                        is_workspace: false,
                    },
                    ManifestFile {
                        content: "".to_string(),
                        file_basename: lib_pkg_version.clone(),
                        file_path: gen_package_path(&lib_pkg_version),
                        is_workspace: false,
                    },
                ]
            }
            Framework::Rust => {
                let cargo_toml_pkg_path = gen_package_path("Cargo.toml");
                let cargo_toml_workspace_path =
                    gen_workspace_path("Cargo.lock");

                if cargo_toml_pkg_path == cargo_toml_workspace_path {
                    // package is not part of workspace with other packages
                    vec![
                        ManifestFile {
                            content: "".to_string(),
                            file_basename: "Cargo.toml".into(),
                            file_path: gen_package_path("Cargo.toml"),
                            is_workspace: false,
                        },
                        ManifestFile {
                            content: "".to_string(),
                            file_basename: "Cargo.lock".into(),
                            file_path: gen_package_path("Cargo.lock"),
                            is_workspace: false,
                        },
                    ]
                } else {
                    // package is part of workspace with other packages so
                    // include workspace root manifest files
                    vec![
                        ManifestFile {
                            content: "".to_string(),
                            file_basename: "Cargo.toml".into(),
                            file_path: gen_package_path("Cargo.toml"),
                            is_workspace: false,
                        },
                        ManifestFile {
                            content: "".to_string(),
                            file_basename: "Cargo.lock".into(),
                            file_path: gen_package_path("Cargo.lock"),
                            is_workspace: false,
                        },
                        ManifestFile {
                            content: "".to_string(),
                            file_basename: "Cargo.lock".into(),
                            file_path: gen_workspace_path("Cargo.lock"),
                            is_workspace: true,
                        },
                    ]
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestFile {
    /// Whether or not to treat this as a workspace manifest
    pub is_workspace: bool,
    /// The file path within the package directory that will be updated
    pub file_path: String,
    /// The base name of the file path
    pub file_basename: String,
    /// The current content of the file
    pub content: String,
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

        let pkg_manifests = framework.manifest_files(pkg);

        let tag = pkg.release.tag.clone().unwrap_or_default();

        UpdaterPackage {
            package_name: pkg.name.clone(),
            workspace_root: pkg.workspace_root.clone(),
            framework,
            manifest_files: pkg_manifests,
            next_version: tag,
        }
    }
}
