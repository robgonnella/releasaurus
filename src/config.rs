//! Configuration loading and parsing for `releasaurus.toml` files.
//!
//! Supports customizable changelog templates and multi-package repositories.
use serde::Deserialize;

use crate::{
    analyzer::config::DEFAULT_BODY, forge::config::DEFAULT_COMMIT_SEARCH_DEPTH,
};

/// Default configuration filename.
pub const DEFAULT_CONFIG_FILE: &str = "releasaurus.toml";

/// Changelog template configuration using Tera syntax.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)] // Use default for missing fields
pub struct ChangelogConfig {
    /// Main changelog body template.
    pub body: String,
    /// Skips including ci commits in changelog (default: false)
    pub skip_ci: bool,
    /// Skips including ci commits in changelog (default: false)
    pub skip_chore: bool,
    /// Skips including miscellaneous commits in changelog (default: false)
    pub skip_miscellaneous: bool,
    /// Includes commit author in default body template (default: false)
    pub include_author: bool,
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        Self {
            body: DEFAULT_BODY.into(),
            skip_ci: false,
            skip_chore: false,
            skip_miscellaneous: false,
            include_author: false,
        }
    }
}

/// Supported release types for updating package files
#[derive(Debug, Default, Clone, Deserialize)]
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

/// Package configuration for multi-package repositories and monorepos.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)] // Use default for missing fields
pub struct PackageConfig {
    /// Package directory path relative to repository root.
    pub path: String,
    /// Release type for determining which version files to update.
    pub release_type: Option<ReleaseType>,
    /// Git tag prefix for this package (e.g., "v" or "api-v").
    pub tag_prefix: Option<String>,
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            path: ".".to_string(),
            release_type: None,
            tag_prefix: None,
        }
    }
}

/// Root configuration structure for `releasaurus.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Maximum number of commits to search for the first release when no
    /// tags exist.
    pub first_release_search_depth: u64,
    /// Changelog generation settings.
    pub changelog: ChangelogConfig,
    /// Packages to manage in this repository (supports monorepos).
    #[serde(rename = "package")]
    pub packages: Vec<PackageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            first_release_search_depth: DEFAULT_COMMIT_SEARCH_DEPTH,
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
