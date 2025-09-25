//! Common trait interface for Git forge platforms (GitHub, GitLab, Gitea).
use async_trait::async_trait;
use std::any::Any;

use crate::{
    analyzer::release::Tag,
    config::Config,
    forge::request::{
        Commit, CreateBranchRequest, CreatePrRequest, ForgeCommit,
        GetPrRequest, PrLabelsRequest, PullRequest, UpdatePrRequest,
    },
    result::Result,
};

#[async_trait]
/// Common interface for Git forge platform operations.
pub trait Forge: Any {
    /// Get forge configuration.
    async fn load_config(&self) -> Result<Config>;
    async fn default_branch(&self) -> Result<String>;
    async fn get_file_contents(&self, path: &str) -> Result<Option<String>>;
    async fn create_release_branch(
        &self,
        req: CreateBranchRequest,
    ) -> Result<Commit>;
    // async fn create_commit(&self, req: CreateCommitRequest) -> Result<String>;
    async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()>;
    /// Get latest tag matching prefix
    async fn get_latest_tag_for_prefix(
        &self,
        prefix: &str,
    ) -> Result<Option<Tag>>;
    /// Returns commit iterator for projected release
    async fn get_commits(
        &self,
        path: &str,
        sha: Option<String>,
    ) -> Result<Vec<ForgeCommit>>;
    /// Get open release pull request if it exists.
    async fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>>;
    /// Get merged release pull request if it exists.
    async fn get_merged_release_pr(&self) -> Result<Option<PullRequest>>;
    /// Create new pull request.
    async fn create_pr(&self, req: CreatePrRequest) -> Result<PullRequest>;
    /// Update existing pull request.
    async fn update_pr(&self, req: UpdatePrRequest) -> Result<()>;
    /// Replace pull request labels.
    async fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()>;
    /// Create release with tag, commit SHA, and release notes.
    async fn create_release(
        &self,
        tag: &str,
        sha: &str,
        notes: &str,
    ) -> Result<()>;
}
