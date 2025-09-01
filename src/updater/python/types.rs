/// A dependency on another package in the same repository
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDependency {
    /// Name of the dependency package
    pub name: String,
    /// Current version requirement
    pub current_version_req: String,
    /// New version requirement to update to
    pub new_version_req: String,
    /// Type of the dependency
    pub dependency_type: DependencyType,
}

impl LocalDependency {
    /// Create a new local dependency
    pub fn new(
        name: String,
        current_version_req: String,
        new_version_req: String,
        dependency_type: DependencyType,
    ) -> Self {
        Self {
            name,
            current_version_req,
            new_version_req,
            dependency_type,
        }
    }
}

/// Types of dependencies across different frameworks
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyType {
    /// Regular runtime dependency
    Runtime,
    /// Development/test dependency
    Development,
    /// Optional dependency
    Optional,
}

/// Python-specific framework metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonMetadata {
    /// Build system (setuptools, poetry, flit, etc.)
    pub build_system: String,
    /// Package manager (pip, poetry, pipenv, etc.)
    pub package_manager: String,
    /// Whether this uses pyproject.toml
    pub uses_pyproject: bool,
}

/// Python-specific package metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonPackageMetadata {
    /// Local dependencies that need updating
    pub local_dependencies: Vec<LocalDependency>,
    /// Python version requirements
    pub python_requires: Option<String>,
}
