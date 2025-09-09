use log::*;
use serde::Deserialize;
use std::{fs, path::Path};

use crate::{analyzer::config::DEFAULT_BODY, result::Result};

const DEFAULT_CONFIG_FILE: &str = "releasaurus.toml";

#[derive(Debug, Clone, Deserialize)]
/// Changelog Configuration allowing you to customize changelog output format
pub struct CliChangelogConfig {
    /// [Tera](https://github.com/Keats/tera) template string allowing you
    /// to modify the format of the generated changelog. A sane default is
    /// provided which includes release versions and commit groupings by type
    ///
    /// default: [`DEFAULT_BODY`]
    pub body: String,
    /// Optional tera template to modify the changelog header
    ///
    /// default: [`None`]
    pub header: Option<String>,
    /// Optional tera template to modify the changelog footer
    ///
    /// default: [`None`]
    pub footer: Option<String>,
}

impl Default for CliChangelogConfig {
    fn default() -> Self {
        Self {
            body: DEFAULT_BODY.to_string(),
            header: None,
            footer: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
/// Package configuration specifying which packages to track as separate
/// releases in this repository
pub struct CliPackageConfig {
    /// Path to a valid directory for the package
    pub path: String,
    /// Optional prefix to use for the package
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
/// Complete configuration for the core
pub struct CliConfig {
    /// [`ChangelogConfig`]
    pub changelog: CliChangelogConfig,
    /// [`Vec<PackageConfig>`]
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

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn loads_defaults() {
        let config = CliConfig::default();
        assert!(!config.changelog.body.is_empty())
    }

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
