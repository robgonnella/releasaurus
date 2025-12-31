//! Configuration loading and parsing for `releasaurus.toml` files.
//!
//! Supports customizable changelog templates and multi-package repositories.
use derive_builder::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    Result, config::package::PackageConfig, error::ReleasaurusError,
    forge::config::DEFAULT_COMMIT_SEARCH_DEPTH,
};

pub mod changelog;
pub mod package;
pub mod prerelease;
pub mod release_type;
pub mod resolver;

use self::prerelease::PrereleaseConfig;

/// Default configuration filename
pub const DEFAULT_CONFIG_FILE: &str = "releasaurus.toml";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Builder)]
#[schemars(rename = "Releasaurus TOML Configuration Schema")]
#[serde(default)]
#[builder(setter(into, strip_option), default)]
/// Configuration properties for `releasaurus.toml`
pub struct Config {
    /// The base branch to target for release PRs, tagging, and releases
    /// defaults to default_branch for repository
    pub base_branch: Option<String>,
    /// Maximum number of commits to search for the first release when no
    /// tags exist
    pub first_release_search_depth: u64,
    /// Generates different release PRs for each package defined in config
    pub separate_pull_requests: bool,
    /// Global prerelease configuration (suffix + strategy). Packages can
    /// override this configuration
    pub prerelease: PrereleaseConfig,
    /// Global config to auto start next release for all packages. Packages
    /// can override this configuration
    pub auto_start_next: Option<bool>,
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
    pub packages: Vec<PackageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_branch: None,
            first_release_search_depth: DEFAULT_COMMIT_SEARCH_DEPTH,
            separate_pull_requests: false,
            prerelease: PrereleaseConfig::default(),
            auto_start_next: None,
            breaking_always_increment_major: true,
            features_always_increment_minor: true,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            changelog: changelog::ChangelogConfig::default(),
            packages: vec![package::PackageConfig::default()],
        }
    }
}

impl Config {
    pub fn base_branch(&self) -> Result<String> {
        self.base_branch
            .clone()
            .ok_or_else(|| ReleasaurusError::BaseBranchNotConfigured)
    }

    pub fn auto_start_next(&self, package: &PackageConfig) -> bool {
        package
            .auto_start_next
            .or(self.auto_start_next)
            .unwrap_or_default()
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

    #[test]
    fn base_branch_returns_value_when_set() {
        let config = Config {
            base_branch: Some("main".into()),
            ..Default::default()
        };

        assert_eq!(config.base_branch().unwrap(), "main");
    }

    #[test]
    fn base_branch_returns_error_when_none() {
        let config = Config {
            base_branch: None,
            ..Default::default()
        };

        assert!(config.base_branch().is_err());
    }

    #[test]
    fn auto_start_next_uses_package_override() {
        let config = Config {
            auto_start_next: Some(false),
            ..Default::default()
        };
        let package = PackageConfig {
            auto_start_next: Some(true),
            ..Default::default()
        };

        assert!(config.auto_start_next(&package));
    }

    #[test]
    fn auto_start_next_uses_global_when_package_not_set() {
        let config = Config {
            auto_start_next: Some(true),
            ..Default::default()
        };
        let package = PackageConfig::default();

        assert!(config.auto_start_next(&package));
    }

    #[test]
    fn auto_start_next_defaults_to_false() {
        let config = Config::default();
        let package = PackageConfig::default();

        assert!(!config.auto_start_next(&package));
    }
}
