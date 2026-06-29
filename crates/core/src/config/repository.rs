use derive_builder::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::result::{ReleasaurusError, Result};

/// Default number of commits to search when processing first release
pub const DEFAULT_COMMIT_SEARCH_DEPTH: usize = 400;
/// Default number of tags to search when looking for previous releases
pub const DEFAULT_TAG_SEARCH_DEPTH: usize = 100;

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

/// Repository configuration (applies to all packages)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Builder)]
#[builder(setter(into, strip_option), default)]
#[serde(default)] // Use default for missing fields
pub struct RepositoryConfig {
    /// The base branch to target for release PRs, tagging, and releases
    /// defaults to default_branch for repository
    pub base_branch: Option<String>,
    /// Maximum number of commits to search for the first release when no
    /// tags exist
    pub first_release_search_depth: usize,
    /// Maximum number of tags to pull when searching for previous releases.
    /// Set to 0 to search all tags
    pub tag_search_depth: usize,
    /// Generates different release PRs for each package defined in config
    pub separate_pull_requests: bool,
    /// Skips targeted commit shas (or prefixes) when generating next version
    /// and changelog. Each value matches any commit whose SHA starts with the
    /// provided value
    pub skip_shas: Vec<String>,
    /// Rewords commit messages for targeted shas when generated changelog.
    /// Each SHA can be a prefix - matches any commit whose SHA starts with the
    /// provided value
    pub reword: Vec<RewordedCommit>,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            base_branch: None,
            first_release_search_depth: DEFAULT_COMMIT_SEARCH_DEPTH,
            tag_search_depth: DEFAULT_TAG_SEARCH_DEPTH,
            separate_pull_requests: false,
            skip_shas: Vec::new(),
            reword: Vec::new(),
        }
    }
}

impl RepositoryConfig {
    pub fn base_branch(&self) -> Result<String> {
        self.base_branch
            .clone()
            .ok_or_else(|| ReleasaurusError::BaseBranchNotConfigured)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_defaults() {
        let config = RepositoryConfig::default();
        assert_eq!(
            config.first_release_search_depth,
            DEFAULT_COMMIT_SEARCH_DEPTH
        );
        assert_eq!(config.tag_search_depth, DEFAULT_TAG_SEARCH_DEPTH);
    }

    #[test]
    fn base_branch_returns_value_when_set() {
        let config = RepositoryConfig {
            base_branch: Some("main".into()),
            ..Default::default()
        };

        assert_eq!(config.base_branch().unwrap(), "main");
    }

    #[test]
    fn base_branch_returns_error_when_none() {
        let config = RepositoryConfig {
            base_branch: None,
            ..Default::default()
        };

        assert!(config.base_branch().is_err());
    }
}
