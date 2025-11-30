//! Implements the Forge trait for Gitea
use log::*;

use crate::{
    Result,
    analyzer::release::Tag,
    config::Config,
    forge::{
        config::RemoteConfig,
        request::{
            Commit, CreateBranchRequest, CreatePrRequest, ForgeCommit,
            GetPrRequest, PrLabelsRequest, PullRequest, UpdatePrRequest,
        },
        traits::Forge,
    },
};

pub struct ForgeManager {
    forge: Box<dyn Forge>,
    remote_config: RemoteConfig,
}

impl ForgeManager {
    /// Create Gitea client with token authentication and API base URL
    /// configuration for self-hosted instances.
    pub fn new(forge: Box<dyn Forge>) -> Self {
        let remote_config = forge.remote_config();
        Self {
            forge,
            remote_config,
        }
    }

    pub fn repo_name(&self) -> String {
        self.forge.repo_name()
    }

    pub fn remote_config(&self) -> RemoteConfig {
        self.remote_config.clone()
    }

    pub fn default_branch(&self) -> String {
        self.forge.default_branch()
    }

    pub async fn get_file_content(&self, path: &str) -> Result<Option<String>> {
        self.forge.get_file_content(path).await
    }

    pub async fn load_config(&self) -> Result<Config> {
        self.forge.load_config().await
    }

    pub async fn get_latest_tag_for_prefix(
        &self,
        prefix: &str,
    ) -> Result<Option<Tag>> {
        self.forge.get_latest_tag_for_prefix(prefix).await
    }

    pub async fn get_commits(
        &self,
        sha: Option<String>,
    ) -> Result<Vec<ForgeCommit>> {
        self.forge.get_commits(sha).await
    }

    pub async fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        self.forge.get_open_release_pr(req).await
    }

    pub async fn get_merged_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        self.forge.get_merged_release_pr(req).await
    }

    pub async fn create_release_branch(
        &self,
        req: CreateBranchRequest,
    ) -> Result<Commit> {
        if self.remote_config.dry_run {
            warn!("dry_run: would create release branch: req: {:#?}", req);
            return Ok(Commit { sha: "fff".into() });
        }
        self.forge.create_release_branch(req).await
    }

    pub async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()> {
        if self.remote_config.dry_run {
            warn!(
                "dry_run: would tag commit: tag_name: {tag_name}, sha: {sha}"
            );
            return Ok(());
        }

        self.forge.tag_commit(tag_name, sha).await
    }

    pub async fn create_pr(&self, req: CreatePrRequest) -> Result<PullRequest> {
        if self.remote_config.dry_run {
            warn!("dry_run: would create PR: req: {:#?}", req);
            return Ok(PullRequest {
                number: 0,
                sha: "fff".into(),
                body: req.body,
            });
        }

        self.forge.create_pr(req).await
    }

    pub async fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        if self.remote_config.dry_run {
            warn!("dry_run: would update PR: req: {:#?}", req);
            return Ok(());
        }
        self.forge.update_pr(req).await
    }

    pub async fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        if self.remote_config.dry_run {
            warn!("dry_run: would replace PR labels: req: {:#?}", req);
            return Ok(());
        }
        self.forge.replace_pr_labels(req).await
    }

    pub async fn create_release(
        &self,
        tag: &str,
        sha: &str,
        notes: &str,
    ) -> Result<()> {
        if self.remote_config.dry_run {
            warn!(
                "dry_run: would create release: tag: {tag}, sha: {sha}, notes {notes}"
            );
            return Ok(());
        }

        self.forge.create_release(tag, sha, notes).await
    }
}
