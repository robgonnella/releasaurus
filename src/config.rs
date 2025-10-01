//! Configuration loading and parsing for `releasaurus.toml` files.
//!
//! Supports customizable changelog templates and multi-package repositories.
use serde::Deserialize;

use crate::analyzer::config::DEFAULT_BODY;

/// Default configuration filename.
pub const DEFAULT_CONFIG_FILE: &str = "releasaurus.toml";

/// Changelog template configuration using Tera syntax.
#[derive(Debug, Clone, Deserialize)]
pub struct ChangelogConfig {
    /// Main changelog body template.
    pub body: String,
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        Self {
            body: DEFAULT_BODY.into(),
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
pub struct PackageConfig {
    /// Package directory path relative to repository root.
    pub path: String,
    /// release type for updating package files
    pub release_type: Option<ReleaseType>,
    /// Optional Git tag prefix for this package.
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
    /// Changelog generation settings.
    pub changelog: ChangelogConfig,
    /// List of packages to manage within this repository.
    #[serde(rename = "package")]
    pub packages: Vec<PackageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            changelog: ChangelogConfig::default(),
            packages: vec![PackageConfig::default()],
        }
    }
}

/// Unit tests for configuration loading.
#[cfg(test)]
mod tests {
    use super::*;

    /// Test default configuration values.
    #[test]
    fn loads_defaults() {
        let config = Config::default();
        assert!(!config.changelog.body.is_empty())
    }
}
