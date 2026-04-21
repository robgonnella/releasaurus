use std::collections::HashMap;

use derive_builder::Builder;
use merge::Merge;
use url::Url;

use crate::config::{
    changelog::{ChangelogConfig, RewordedCommit},
    prerelease::{PrereleaseConfig, PrereleaseStrategy},
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

/// Package name used as the key in override and config maps.
pub type PackageName = String;

#[derive(Debug, Clone, Default)]
pub struct CommitModifiers {
    /// Commit sha (or prefix) to skip when calculating next version and
    /// generating changelog. Matches any commit whose SHA starts with the
    /// provided value
    pub skip_shas: Vec<String>,
    /// Rewords a commit message when generating changelog. The SHA can be a
    /// prefix - matches any commit whose SHA starts with the provided value.
    pub reword: Vec<RewordedCommit>,
}

/// Fully resolved runtime configuration for the release pipeline.
///
/// Produced by [`Resolver::resolve`][crate::resolver::Resolver::resolve]
/// from the loaded TOML config, CLI overrides, and forge metadata.
/// All optional values have been resolved to concrete defaults.
#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct ResolvedConfig {
    pub repo_name: String,
    pub base_branch: String,
    pub release_link_base_url: Url,
    pub compare_link_base_url: Url,
    pub package_overrides: HashMap<PackageName, PackageOverrides>,
    pub global_overrides: GlobalOverrides,
    pub commit_modifiers: CommitModifiers,
    pub first_release_search_depth: u64,
    pub separate_pull_requests: bool,
    pub prerelease: PrereleaseConfig,
    pub auto_start_next: Option<bool>,
    pub breaking_always_increment_major: bool,
    pub features_always_increment_minor: bool,
    pub custom_major_increment_regex: Option<String>,
    pub custom_minor_increment_regex: Option<String>,
    pub changelog: ChangelogConfig,
}
