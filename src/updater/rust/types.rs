use std::path::PathBuf;

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
    /// Build-time dependency
    Build,
}

// use crate::updater::types::LocalDependency;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Rust-specific framework metadata
pub struct RustMetadata {
    /// Whether this is a Cargo workspace
    pub is_workspace: bool,
    /// Workspace root path if this is a workspace member
    pub workspace_root: Option<PathBuf>,
    /// Package manager (always Cargo for Rust)
    pub package_manager: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Rust-specific package metadata
pub struct RustPackageMetadata {
    /// Whether this package is a workspace member
    pub is_workspace_member: bool,
    /// Whether this package is the workspace root
    pub is_workspace_root: bool,
    // /// Local workspace dependencies that need updating
    pub local_dependencies: Vec<LocalDependency>,
}
