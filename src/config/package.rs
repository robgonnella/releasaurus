use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::release_type::ReleaseType;

/// Package configuration for multi-package repositories and monorepos
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)] // Use default for missing fields
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
    /// Prerelease identifier (e.g., "alpha", "beta", "rc")
    pub prerelease: Option<String>,
    /// Whether to append .1, .2, etc. to prerelease versions
    pub prerelease_version: Option<bool>,
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
            prerelease_version: None,
            additional_paths: None,
            additional_manifest_files: None,
            breaking_always_increment_major: None,
            features_always_increment_minor: None,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
        }
    }
}
