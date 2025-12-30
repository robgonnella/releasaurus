use derive_builder::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::analyzer::config::DEFAULT_BODY;

/// Rewords messages in changelog for targeted commit shas
#[derive(
    Debug, Clone, Default, JsonSchema, Serialize, Deserialize, Builder,
)]
#[builder(setter(into))]
pub struct RewordedCommit {
    /// Sha (or prefix) of the commit to reword. Matches any commit whose SHA
    /// starts with this value
    pub sha: String,
    /// The new message to display in changelog
    pub message: String,
}

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
    /// Skips targeted commit shas (or prefixes) when generating next version
    /// and changelog. Each value matches any commit whose SHA starts with the
    /// provided value
    pub skip_shas: Option<Vec<String>>,
    /// Rewords commit messages for targeted shas when generated changelog.
    /// Each SHA can be a prefix - matches any commit whose SHA starts with the
    /// provided value
    pub reword: Option<Vec<RewordedCommit>>,
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
            skip_shas: None,
            reword: None,
            include_author: false,
        }
    }
}
