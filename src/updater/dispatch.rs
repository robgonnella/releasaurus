//! Static dispatch updater enum for zero-cost abstraction over package updaters.

use crate::{
    Result,
    config::release_type::ReleaseType,
    forge::request::FileChange,
    updater::{
        generic::updater::GenericUpdater, java::updater::JavaUpdater,
        manager::UpdaterPackage, node::updater::NodeUpdater,
        php::updater::PhpUpdater, python::updater::PythonUpdater,
        ruby::updater::RubyUpdater, rust::updater::RustUpdater,
        traits::PackageUpdater,
    },
};

/// Language-specific updater with static dispatch for optimal performance.
///
/// This enum wraps concrete updater implementations, allowing the compiler to
/// use static dispatch instead of dynamic dispatch (vtable lookups), resulting
/// in better inlining and optimization opportunities.
pub enum Updater {
    /// Generic updater for projects without specific language support
    Generic(GenericUpdater),
    /// Java/Maven updater for pom.xml files
    Java(JavaUpdater),
    /// Node.js updater for package.json, package-lock.json, and yarn.lock
    Node(NodeUpdater),
    /// PHP updater for composer.json
    Php(PhpUpdater),
    /// Python updater for setup.py, pyproject.toml, etc.
    Python(PythonUpdater),
    /// Ruby updater for Gemfile, gemspec, version.rb
    Ruby(RubyUpdater),
    /// Rust updater for Cargo.toml and Cargo.lock
    Rust(RustUpdater),
}

impl Updater {
    /// Create a new updater instance for the given release type.
    ///
    /// # Example
    /// ```
    /// let updater = Updater::new(ReleaseType::Node);
    /// ```
    pub fn new(release_type: ReleaseType) -> Self {
        match release_type {
            ReleaseType::Generic => Updater::Generic(GenericUpdater::new()),
            ReleaseType::Java => Updater::Java(JavaUpdater::new()),
            ReleaseType::Node => Updater::Node(NodeUpdater::new()),
            ReleaseType::Php => Updater::Php(PhpUpdater::new()),
            ReleaseType::Python => Updater::Python(PythonUpdater::new()),
            ReleaseType::Ruby => Updater::Ruby(RubyUpdater::new()),
            ReleaseType::Rust => Updater::Rust(RustUpdater::new()),
        }
    }

    /// Update package version files with static dispatch.
    ///
    /// This method dispatches to the appropriate language-specific updater
    /// using static dispatch, avoiding the overhead of trait objects and
    /// enabling compiler optimizations like inlining.
    pub fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        match self {
            Updater::Generic(updater) => {
                updater.update(package, workspace_packages)
            }
            Updater::Java(updater) => {
                updater.update(package, workspace_packages)
            }
            Updater::Node(updater) => {
                updater.update(package, workspace_packages)
            }
            Updater::Php(updater) => {
                updater.update(package, workspace_packages)
            }
            Updater::Python(updater) => {
                updater.update(package, workspace_packages)
            }
            Updater::Ruby(updater) => {
                updater.update(package, workspace_packages)
            }
            Updater::Rust(updater) => {
                updater.update(package, workspace_packages)
            }
        }
    }
}

impl std::fmt::Debug for Updater {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Updater::Generic(_) => write!(f, "Updater::Generic"),
            Updater::Java(_) => write!(f, "Updater::Java"),
            Updater::Node(_) => write!(f, "Updater::Node"),
            Updater::Php(_) => write!(f, "Updater::Php"),
            Updater::Python(_) => write!(f, "Updater::Python"),
            Updater::Ruby(_) => write!(f, "Updater::Ruby"),
            Updater::Rust(_) => write!(f, "Updater::Rust"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_updater_for_each_release_type() {
        let types = vec![
            ReleaseType::Generic,
            ReleaseType::Java,
            ReleaseType::Node,
            ReleaseType::Php,
            ReleaseType::Python,
            ReleaseType::Ruby,
            ReleaseType::Rust,
        ];

        for release_type in types {
            let updater = Updater::new(release_type);
            // If we got here without panicking, the updater was created successfully
            assert!(matches!(
                updater,
                Updater::Generic(_)
                    | Updater::Java(_)
                    | Updater::Node(_)
                    | Updater::Php(_)
                    | Updater::Python(_)
                    | Updater::Ruby(_)
                    | Updater::Rust(_)
            ));
        }
    }
}
