//! Configuration loading and parsing for `releasaurus.toml` files.
//!
//! Supports customizable changelog templates and multi-package repositories.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    analyzer::config::DEFAULT_BODY, forge::config::DEFAULT_COMMIT_SEARCH_DEPTH,
};

/// Default configuration filename
pub const DEFAULT_CONFIG_FILE: &str = "releasaurus.toml";

/// Changelog configuration (applies to all packages)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)] // Use default for missing fields
pub struct ChangelogConfig {
    /// Main changelog body template.
    pub body: String,
    /// Skips including ci commits in changelog
    pub skip_ci: bool,
    /// Skips including ci commits in changelog
    pub skip_chore: bool,
    /// Skips including miscellaneous commits in changelog
    pub skip_miscellaneous: bool,
    /// Skips including merge commits in changelog
    pub skip_merge_commits: bool,
    /// Skips including release commits in changelog
    pub skip_release_commits: bool,
    /// Includes commit author name in default body template
    pub include_author: bool,
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        Self {
            body: DEFAULT_BODY.into(),
            skip_ci: false,
            skip_chore: false,
            skip_miscellaneous: false,
            skip_merge_commits: true,
            skip_release_commits: true,
            include_author: false,
        }
    }
}

/// Supported release types for updating package manifest files
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseType {
    #[default]
    Generic,
    Node,
    Rust,
    Python,
    Php,
    Ruby,
    Java,
}

/// Package configuration for multi-package repositories and monorepos
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)] // Use default for missing fields
pub struct PackageConfig {
    /// Name for this package (default derived from path if not provided)
    pub name: String,
    /// Path to the workspace root directory for this package relative to the repository root
    pub workspace_root: String,
    /// Path to package directory relative to workspace_root
    pub path: String,
    /// [`ReleaseType`] type for determining which version files to update.
    pub release_type: Option<ReleaseType>,
    /// Git tag prefix for this package (e.g., "v" or "api-v").
    pub tag_prefix: Option<String>,
    /// Prerelease identifier (e.g., "alpha", "beta", "rc").
    pub prerelease: Option<String>,
    /// Additional directory paths to include commits from
    pub additional_paths: Option<Vec<String>>,
    /// Always increments major version on breaking commits
    pub breaking_always_increment_major: Option<bool>,
    /// Always increments minor version on feature commits
    pub features_always_increment_minor: Option<bool>,
    /// Custom commit type regex matcher to increment major version
    pub custom_major_increment_regex: Option<String>,
    /// Custom commit type regex matcher to increment minor version
    pub custom_minor_increment_regex: Option<String>,
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            name: "".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: None,
            tag_prefix: None,
            prerelease: None,
            additional_paths: None,
            breaking_always_increment_major: None,
            features_always_increment_minor: None,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
        }
    }
}

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
    /// Always increments major version on breaking commits
    pub breaking_always_increment_major: bool,
    /// Always increments minor version on feature commits
    pub features_always_increment_minor: bool,
    /// Custom commit type regex matcher to increment major version
    pub custom_major_increment_regex: Option<String>,
    /// Custom commit type regex matcher to increment minor version
    pub custom_minor_increment_regex: Option<String>,
    /// Changelog generation settings.
    pub changelog: ChangelogConfig,
    /// Packages to manage in this repository (supports monorepos)
    #[serde(rename = "package")]
    pub packages: Vec<PackageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            first_release_search_depth: DEFAULT_COMMIT_SEARCH_DEPTH,
            separate_pull_requests: false,
            prerelease: None,
            breaking_always_increment_major: true,
            features_always_increment_minor: true,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            changelog: ChangelogConfig::default(),
            packages: vec![PackageConfig::default()],
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
