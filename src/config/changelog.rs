use derive_builder::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::analyzer::config::DEFAULT_BODY;

/// Changelog configuration (applies to all packages)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Builder)]
#[builder(setter(into, strip_option), default)]
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
