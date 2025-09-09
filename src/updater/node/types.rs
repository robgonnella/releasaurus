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

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(unused)]
/// Types of dependencies across different frameworks
pub enum DependencyType {
    /// Regular runtime dependency
    Runtime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(unused)]
/// Node.js-specific framework metadata
pub struct NodeMetadata {
    /// Whether this is a monorepo (workspaces/lerna)
    pub is_monorepo: bool,
    /// Monorepo root path if this is a workspace member
    pub monorepo_root: Option<PathBuf>,
    /// Package manager (npm, yarn, pnpm, etc.)
    pub package_manager: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(unused)]
/// Node.js-specific package metadata
pub struct NodePackageMetadata {
    /// Whether this package is part of a monorepo
    pub is_workspace_member: bool,
    /// Whether this package is the monorepo root
    pub is_monorepo_root: bool,
    /// Local workspace dependencies that need updating
    pub local_dependencies: Vec<LocalDependency>,
}
