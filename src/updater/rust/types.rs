use std::path::PathBuf;

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
    /// Build-time dependency
    Build,
}

// use crate::updater::types::LocalDependency;

/// Rust-specific framework metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustMetadata {
    /// Whether this is a Cargo workspace
    pub is_workspace: bool,
    /// Workspace root path if this is a workspace member
    pub workspace_root: Option<PathBuf>,
    /// Package manager (always Cargo for Rust)
    pub package_manager: String,
}

/// Rust-specific package metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustPackageMetadata {
    /// Whether this package is a workspace member
    pub is_workspace_member: bool,
    /// Whether this package is the workspace root
    pub is_workspace_root: bool,
    // /// Local workspace dependencies that need updating
    pub local_dependencies: Vec<LocalDependency>,
}
