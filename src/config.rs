//! Configuration loading and parsing for `releasaurus.toml` files.
//!
//! Supports customizable changelog templates and multi-package repositories.

use log::*;
use serde::Deserialize;
use std::{fs, path::Path};

use crate::{
    analyzer::config::{DEFAULT_BODY, DEFAULT_FOOTER},
    result::Result,
};

/// Default configuration filename.
const DEFAULT_CONFIG_FILE: &str = "releasaurus.toml";

/// Changelog template configuration using Tera syntax.
#[derive(Debug, Clone, Deserialize)]
pub struct CliChangelogConfig {
    /// Main changelog body template.
    pub body: String,
    /// Optional header template for the changelog.
    pub header: Option<String>,
    /// Optional footer template for the changelog.
    pub footer: Option<String>,
}

impl Default for CliChangelogConfig {
    fn default() -> Self {
        Self {
            body: DEFAULT_BODY.into(),
            header: None,
            footer: Some(DEFAULT_FOOTER.into()),
        }
    }
}

/// Package configuration for multi-package repositories and monorepos.
#[derive(Debug, Clone, Deserialize)]
pub struct CliPackageConfig {
    /// Package directory path relative to repository root.
    pub path: String,
    /// Optional Git tag prefix for this package.
    pub tag_prefix: Option<String>,
}

impl Default for CliPackageConfig {
    fn default() -> Self {
        Self {
            path: ".".to_string(),
            tag_prefix: None,
        }
    }
}

/// Root configuration structure for `releasaurus.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CliConfig {
    /// Changelog generation settings.
    pub changelog: CliChangelogConfig,
    /// List of packages to manage within this repository.
    #[serde(rename = "package")]
    pub packages: Vec<CliPackageConfig>,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            changelog: CliChangelogConfig::default(),
            packages: vec![CliPackageConfig::default()],
        }
    }
}

/// Load configuration from `releasaurus.toml` or return defaults.
pub fn load_config(dir: Option<&Path>) -> Result<CliConfig> {
    let project = dir.unwrap_or(Path::new("."));
    let config_path = project.join(DEFAULT_CONFIG_FILE);
    let exists = std::fs::exists(config_path.clone())?;

    // search for config file walking up ancestors as necessary
    if exists {
        info!("found config file: {}", config_path.display());
        if let Ok(content) = fs::read_to_string(config_path) {
            let cli_config: CliConfig = toml::from_str(&content)?;
            return Ok(cli_config);
        }
    }

    // otherwise return default config
    info!(
        "no configuration file found for {DEFAULT_CONFIG_FILE}: using default config"
    );
    Ok(CliConfig::default())
}

/// Unit tests for configuration loading.
#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    /// Test default configuration values.
    #[test]
    fn loads_defaults() {
        let config = CliConfig::default();
        assert!(!config.changelog.body.is_empty())
    }

    /// Test loading from TOML file.
    #[test]
    fn loads_config_file() {
        let tmp = TempDir::new().unwrap();

        let content = r#"
[[package]]
path = "./some/path"
"#;

        fs::write(tmp.path().join(DEFAULT_CONFIG_FILE), content.as_bytes())
            .unwrap();

        let result = load_config(Some(tmp.path()));
        assert!(result.is_ok(), "failed to load config file");

        let config = result.unwrap();

        assert_eq!(config.packages.len(), 1, "packages length should be 1");
        assert_eq!(
            config.packages[0].path, "./some/path",
            "packages path should be ./some/path"
        );
    }

    /// Test fallback to defaults when no file exists.
    #[test]
    fn loads_default_config() {
        let tmp = TempDir::new().unwrap();

        let result = load_config(Some(tmp.path()));
        assert!(result.is_ok(), "failed to load config file");

        let config = result.unwrap();
        assert_eq!(config.packages.len(), 1, "packages length should be 1");
        assert_eq!(
            config.packages[0].path, ".",
            "package path should be \".\""
        );
    }
}
