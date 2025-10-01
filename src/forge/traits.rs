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
pub trait FileLoader: Sync {
    async fn get_file_content(&self, path: &str) -> Result<Option<String>>;
}

#[async_trait]
/// Common interface for Git forge platform operations.
pub trait Forge: Any {
    fn repo_name(&self) -> String;
    async fn load_config(&self) -> Result<Config>;
    async fn default_branch(&self) -> Result<String>;
    async fn create_release_branch(
        &self,
        req: CreateBranchRequest,
    ) -> Result<Commit>;
    async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()>;
    async fn get_latest_tag_for_prefix(
        &self,
        prefix: &str,
    ) -> Result<Option<Tag>>;
    async fn get_commits(
        &self,
        path: &str,
        sha: Option<String>,
    ) -> Result<Vec<ForgeCommit>>;
    async fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>>;
    async fn get_merged_release_pr(&self) -> Result<Option<PullRequest>>;
    async fn create_pr(&self, req: CreatePrRequest) -> Result<PullRequest>;
    async fn update_pr(&self, req: UpdatePrRequest) -> Result<()>;
    async fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()>;
    async fn create_release(
        &self,
        tag: &str,
        sha: &str,
        notes: &str,
    ) -> Result<()>;
}
