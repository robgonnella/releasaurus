//! Common trait interface for Git forge platforms (GitHub, GitLab, Gitea).
use async_trait::async_trait;
use std::any::Any;

#[cfg(test)]
use mockall::automock;

use crate::{
    analyzer::release::Tag,
    config::Config,
    forge::{
        config::RemoteConfig,
        request::{
            Commit, CreateBranchRequest, CreatePrRequest, ForgeCommit,
            GetPrRequest, PrLabelsRequest, PullRequest, UpdatePrRequest,
        },
    },
    result::Result,
};

/// Common interface for Git forge platform operations including repository
/// access, PR management, tagging, and release publishing.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait Forge: Any {
    /// Get repository name from configuration.
    fn repo_name(&self) -> String;
    /// Get remote configuration details including host, owner, and auth.
    fn remote_config(&self) -> RemoteConfig;
    /// Fetch the default branch name (e.g., "main" or "master").
    fn default_branch(&self) -> String;
    /// Load releasaurus.toml configuration from repository root.
    async fn load_config(&self) -> Result<Config>;
    /// Fetch file content from repository by path, returning None if file
    /// doesn't exist.
    async fn get_file_content(&self, path: &str) -> Result<Option<String>>;
    /// Create a new branch with file changes and return the commit SHA.
    async fn create_release_branch(
        &self,
        req: CreateBranchRequest,
    ) -> Result<Commit>;
    /// Create a git tag pointing to a specific commit SHA.
    async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()>;
    /// Find the most recent tag matching the given prefix (e.g., "v" or
    /// "api-v").
    async fn get_latest_tag_for_prefix(
        &self,
        prefix: &str,
    ) -> Result<Option<Tag>>;
    /// Fetch commits for a package path, optionally starting from a specific
    /// SHA.
    async fn get_commits(
        &self,
        sha: Option<String>,
    ) -> Result<Vec<ForgeCommit>>;
    /// Find an open release PR matching the given branch criteria.
    async fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>>;
    /// Find the most recently merged release PR with pending label.
    async fn get_merged_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>>;
    /// Create a new pull request and return its details.
    async fn create_pr(&self, req: CreatePrRequest) -> Result<PullRequest>;
    /// Update an existing pull request's title and body.
    async fn update_pr(&self, req: UpdatePrRequest) -> Result<()>;
    /// Replace all labels on a pull request with the provided set.
    async fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()>;
    /// Publish a release on the forge platform with notes and tag reference.
    async fn create_release(
        &self,
        tag: &str,
        sha: &str,
        notes: &str,
    ) -> Result<()>;
}
