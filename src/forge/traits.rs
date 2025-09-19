//! Common trait interface for Git forge platforms (GitHub, GitLab, Gitea).
use std::any::Any;

use crate::{
    analyzer::types::Tag,
    forge::{
        config::RemoteConfig,
        types::{
            CreatePrRequest, GetPrRequest, PrLabelsRequest, ReleasePullRequest,
            UpdatePrRequest,
        },
    },
    result::Result,
};

/// Common interface for Git forge platform operations.
pub trait Forge: Any {
    /// Get forge configuration.
    fn config(&self) -> &RemoteConfig;
    /// Get latest tag matching prefix
    fn get_latest_tag_for_prefix(&self, prefix: &str) -> Result<Option<Tag>>;
    /// Get open release pull request if it exists.
    fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<ReleasePullRequest>>;
    /// Get merged release pull request if it exists.
    fn get_merged_release_pr(&self) -> Result<Option<ReleasePullRequest>>;
    /// Create new pull request.
    fn create_pr(&self, req: CreatePrRequest) -> Result<ReleasePullRequest>;
    /// Update existing pull request.
    fn update_pr(&self, req: UpdatePrRequest) -> Result<()>;
    /// Replace pull request labels.
    fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()>;
    /// Create release with tag, commit SHA, and release notes.
    fn create_release(&self, tag: &str, sha: &str, notes: &str) -> Result<()>;
}
