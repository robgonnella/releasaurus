use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::{
    changelog::ChangelogConfig,
    versioning::{
        DEFAULT_BREAKING_ALWAYS_INCREMENT_MAJOR,
        DEFAULT_FEAT_ALWAYS_INCREMENT_MINOR, DEFAULT_SKIP_MERGE_COMMITS,
        DEFAULT_VERSION_TYPE, NAMED_PARSERS, VersioningConfig,
    },
};

fn default_versioning() -> VersioningConfig {
    VersioningConfig {
        version_type: Some(DEFAULT_VERSION_TYPE),
        prerelease: None,
        auto_start_next: Some(false),
        breaking_always_increment_major: Some(
            DEFAULT_BREAKING_ALWAYS_INCREMENT_MAJOR,
        ),
        features_always_increment_minor: Some(
            DEFAULT_FEAT_ALWAYS_INCREMENT_MINOR,
        ),
        custom_major_increment_regex: None,
        custom_minor_increment_regex: None,
        skip_merge_commits: Some(DEFAULT_SKIP_MERGE_COMMITS),
        named_parsers: Some(NAMED_PARSERS.clone()),
        custom_parsers: None,
    }
}

/// Default configuration applied to every package
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)] // Use default for missing fields
pub struct DefaultsConfig {
    /// Default versioning config. Packages can override this configuration
    #[schemars(default = "default_versioning")]
    pub versioning: Option<VersioningConfig>,
    /// Default changelog generation settings applied to all packages.
    /// Packages can override this configuration
    pub changelog: Option<ChangelogConfig>,
}
