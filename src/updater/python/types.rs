#[derive(Debug, Clone, PartialEq, Eq)]
/// A dependency on another package in the same repository
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

#[allow(unused)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(unused)]
/// Types of dependencies across different frameworks
pub enum DependencyType {
    /// Regular runtime dependency
    Runtime,
    /// Development/test dependency
    Development,
    /// Optional dependency
    Optional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(unused)]
/// Python-specific framework metadata
pub struct PythonMetadata {
    /// Build system (setuptools, poetry, flit, etc.)
    pub build_system: String,
    /// Package manager (pip, poetry, pipenv, etc.)
    pub package_manager: String,
    /// Whether this uses pyproject.toml
    pub uses_pyproject: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(unused)]
/// Python-specific package metadata
pub struct PythonPackageMetadata {
    /// Local dependencies that need updating
    pub local_dependencies: Vec<LocalDependency>,
    /// Python version requirements
    pub python_requires: Option<String>,
}
