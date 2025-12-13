//! Configuration loading and parsing for `releasaurus.toml` files.
//!
//! Supports customizable changelog templates and multi-package repositories.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::forge::config::DEFAULT_COMMIT_SEARCH_DEPTH;

pub mod changelog;
pub mod package;
pub mod release_type;

/// Default configuration filename
pub const DEFAULT_CONFIG_FILE: &str = "releasaurus.toml";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Releasaurus TOML Configuration Schema")]
#[serde(default)]
/// Configuration properties for `releasaurus.toml`
pub struct Config {
    /// Maximum number of commits to search for the first release when no
    /// tags exist.
    pub first_release_search_depth: u64,
    /// Generates different release PRs for each package defined in config
    pub separate_pull_requests: bool,
    /// Global prerelease identifier (e.g., "alpha", "beta", "rc").
    /// Can be overridden per package
    pub prerelease: Option<String>,
    /// Whether to append .1, .2, etc. to prerelease versions
    /// Can be overridden per package
    pub prerelease_version: bool,
    /// Always increments major version on breaking commits
    pub breaking_always_increment_major: bool,
    /// Always increments minor version on feature commits
    pub features_always_increment_minor: bool,
    /// Custom commit type regex matcher to increment major version
    pub custom_major_increment_regex: Option<String>,
    /// Custom commit type regex matcher to increment minor version
    pub custom_minor_increment_regex: Option<String>,
    /// Changelog generation settings.
    pub changelog: changelog::ChangelogConfig,
    /// Packages to manage in this repository (supports monorepos)
    #[serde(rename = "package")]
    pub packages: Vec<package::PackageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            first_release_search_depth: DEFAULT_COMMIT_SEARCH_DEPTH,
            separate_pull_requests: false,
            prerelease: None,
            prerelease_version: true,
            breaking_always_increment_major: true,
            features_always_increment_minor: true,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            changelog: changelog::ChangelogConfig::default(),
            packages: vec![package::PackageConfig::default()],
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::forge::config::DEFAULT_COMMIT_SEARCH_DEPTH;

    use super::*;

    #[test]
    fn loads_defaults() {
        let config = Config::default();
        assert!(!config.changelog.body.is_empty());
        assert_eq!(
            config.first_release_search_depth,
            DEFAULT_COMMIT_SEARCH_DEPTH
        );
    }
}
