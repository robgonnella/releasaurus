use derive_builder::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    Result,
    analyzer::config::AnalyzerConfig,
    config::{prerelease::PrereleaseConfig, release_type::ReleaseType},
    error::ReleasaurusError,
};

pub const DEFAULT_TAG_PREFIX: &str = "v";

/// Package configuration for multi-package repositories and monorepos
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Builder)]
#[serde(default)] // Use default for missing fields
#[builder(setter(into, strip_option), default)]
pub struct PackageConfig {
    /// Name for this package (default derived from path if not provided)
    pub name: String,
    /// Path to the workspace root directory for this package relative to the
    /// repository root
    pub workspace_root: String,
    /// Path to package directory relative to workspace_root
    pub path: String,
    /// [`ReleaseType`] type for determining which version files to update
    pub release_type: Option<ReleaseType>,
    /// Git tag prefix for this package (e.g., "v" or "api-v")
    pub tag_prefix: Option<String>,
    /// Optional prerelease configuration that overrides global settings
    pub prerelease: Option<PrereleaseConfig>,
    /// Auto starts next release for this package by performing a patch version
    /// update to version files and pushing a "chore" commit to the base_branch
    pub auto_start_next: Option<bool>,
    /// Additional directory paths to include commits from
    pub additional_paths: Option<Vec<String>>,
    /// Additional paths generic version manifest files to update. Paths must
    /// be relative to the package path
    pub additional_manifest_files: Option<Vec<String>>,
    /// Always increments major version on breaking commits
    pub breaking_always_increment_major: Option<bool>,
    /// Always increments minor version on feature commits
    pub features_always_increment_minor: Option<bool>,
    /// Custom commit type regex matcher to increment major version
    pub custom_major_increment_regex: Option<String>,
    /// Custom commit type regex matcher to increment minor version
    pub custom_minor_increment_regex: Option<String>,
    /// derived from all other provided config
    #[serde(skip)]
    pub analyzer_config: AnalyzerConfig,
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
            auto_start_next: None,
            additional_paths: None,
            additional_manifest_files: None,
            breaking_always_increment_major: None,
            features_always_increment_minor: None,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            analyzer_config: AnalyzerConfig::default(),
        }
    }
}

impl PackageConfig {
    pub fn tag_prefix(&self) -> Result<String> {
        self.tag_prefix.clone().ok_or_else(|| {
            ReleasaurusError::invalid_config(format!(
                "failed to resolve tag prefix for package: {}",
                self.name
            ))
        })
    }
}
