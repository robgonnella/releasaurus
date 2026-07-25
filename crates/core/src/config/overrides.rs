/**
 * These are optional and provided as a way for consumers to dynamically
 * modify config based on cli options
 */
use merge::Merge;
use std::collections::HashMap;

use crate::config::{
    prerelease::PrereleaseStrategy, repository::RewordedCommit,
};

/// Runtime overrides for a specific named package.
///
/// Applied on top of global overrides and the package's TOML
/// config. Only `Some` values take effect; `None` means "use the
/// resolved default."
#[derive(Debug, Clone, Merge)]
pub struct PackageOverrides {
    #[merge(strategy = merge::option::overwrite_none)]
    pub tag_prefix: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub prerelease_suffix: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub prerelease_strategy: Option<PrereleaseStrategy>,
}

/// Runtime overrides that apply to all packages.
///
/// Typically sourced from CLI flags. Only `Some` values take
/// effect.
#[derive(Debug, Clone, Default, Merge)]
pub struct GlobalOverrides {
    #[merge(strategy = merge::option::overwrite_none)]
    pub base_branch: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub tag_prefix: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub prerelease_suffix: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub prerelease_strategy: Option<PrereleaseStrategy>,
}

#[derive(Debug, Clone, Default)]
pub struct CommitModifiers {
    /// Commit sha (or prefix) to skip when calculating next version and
    /// generating changelog. Matches any commit whose SHA starts with the
    /// provided value
    pub skip_shas: Vec<String>,
    /// Rewords commit messages for targeted shas when generated changelog.
    /// Each SHA can be a prefix - matches any commit whose SHA starts with the
    /// provided value
    pub reword: Vec<RewordedCommit>,
}

/// Package name used as the key in override and config maps.
pub type PackageName = String;

/// Lookup table for finding package overrides by package name
pub type PackageOverridesHash = HashMap<PackageName, PackageOverrides>;
