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

/// Types of dependencies across different frameworks
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyType {
    /// Regular runtime dependency
    Runtime,
}

/// Node.js-specific framework metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeMetadata {
    /// Whether this is a monorepo (workspaces/lerna)
    pub is_monorepo: bool,
    /// Monorepo root path if this is a workspace member
    pub monorepo_root: Option<PathBuf>,
    /// Package manager (npm, yarn, pnpm, etc.)
    pub package_manager: String,
}

/// Node.js-specific package metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodePackageMetadata {
    /// Whether this package is part of a monorepo
    pub is_workspace_member: bool,
    /// Whether this package is the monorepo root
    pub is_monorepo_root: bool,
    /// Local workspace dependencies that need updating
    pub local_dependencies: Vec<LocalDependency>,
}
