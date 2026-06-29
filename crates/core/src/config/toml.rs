//! Configuration loading and parsing for `releasaurus.toml` files.
//!
//! Supports customizable changelog templates and multi-package repositories.
use derive_builder::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::{
    global::GlobalConfig, package::PackageConfig, repository::RepositoryConfig,
};

/// Default configuration filename
pub const DEFAULT_CONFIG_FILE: &str = "releasaurus.toml";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Builder)]
#[schemars(rename = "Releasaurus TOML Configuration Schema")]
#[serde(default)]
#[builder(setter(into, strip_option), default)]
/// Configuration properties for `releasaurus.toml`
pub struct Config {
    /// Repository configuration
    pub repository: RepositoryConfig,
    /// Global configuration that applies to all packages
    pub global: GlobalConfig,
    /// Packages to manage in this repository (supports monorepos)
    #[serde(rename = "package")]
    pub packages: Vec<PackageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            repository: RepositoryConfig::default(),
            global: GlobalConfig::default(),
            packages: vec![PackageConfig::default()],
        }
    }
}
