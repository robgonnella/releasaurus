use async_trait::async_trait;
use secrecy::SecretString;
use url::Url;

use crate::{
    config::Config,
    forge::{
        config::{RepoUrl, TokenVar},
        gitea::Gitea,
        request::{
            Commit, CreateCommitRequest, CreatePrRequest,
            CreateReleaseBranchRequest, ForgeCommit, GetFileContentRequest,
            GetPrRequest, PrLabelsRequest, PullRequest, ReleaseByTagResponse,
            Tag, UpdatePrRequest,
        },
        traits::Forge,
    },
    result::Result,
};

pub struct Forgejo {
    gitea: Gitea,
}

impl Forgejo {
    pub async fn new(
        url: RepoUrl,
        token: Option<SecretString>,
    ) -> Result<Self> {
        let gitea = Gitea::new(url, token, Some(TokenVar::Forgejo)).await?;

        Ok(Self { gitea })
    }
}

#[async_trait]
impl Forge for Forgejo {
    fn repo_name(&self) -> String {
        self.gitea.repo_name()
    }

    fn release_link_base_url(&self) -> Url {
        self.gitea.release_link_base_url()
    }

    fn compare_link_base_url(&self) -> Url {
        self.gitea.compare_link_base_url()
    }

    fn default_branch(&self) -> String {
        self.gitea.default_branch()
    }

    fn set_commit_search_depth(&mut self, depth: usize) {
        self.gitea.set_commit_search_depth(depth)
    }

    fn set_tag_search_depth(&mut self, depth: usize) {
        self.gitea.set_tag_search_depth(depth)
    }

    async fn get_file_content(
        &self,
        req: GetFileContentRequest,
    ) -> Result<Option<String>> {
        self.gitea.get_file_content(req).await
    }

    async fn load_config(&self, branch: Option<String>) -> Result<Config> {
        self.gitea.load_config(branch).await
    }

    async fn get_release_by_tag(
        &self,
        tag: &str,
    ) -> Result<ReleaseByTagResponse> {
        self.gitea.get_release_by_tag(tag).await
    }

    // We return only tags that matches the prefix AND are ancestors of
    // the target base branch.
    async fn get_latest_tags_for_prefix(
        &self,
        prefix: &str,
        branch: &str,
    ) -> Result<Vec<Tag>> {
        self.gitea.get_latest_tags_for_prefix(prefix, branch).await
    }

    async fn get_commits(
        &self,
        branch: Option<String>,
        sha: Option<String>,
    ) -> Result<Vec<ForgeCommit>> {
        self.gitea.get_commits(branch, sha).await
    }

    async fn create_release_branch(
        &self,
        req: CreateReleaseBranchRequest,
    ) -> Result<Commit> {
        self.gitea.create_release_branch(req).await
    }

    async fn create_commit(&self, req: CreateCommitRequest) -> Result<Commit> {
        self.gitea.create_commit(req).await
    }

    async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()> {
        self.gitea.tag_commit(tag_name, sha).await
    }

    async fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        self.gitea.get_open_release_pr(req).await
    }

    async fn get_merged_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        self.gitea.get_merged_release_pr(req).await
    }

    async fn create_pr(&self, req: CreatePrRequest) -> Result<PullRequest> {
        self.gitea.create_pr(req).await
    }

    async fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        self.gitea.update_pr(req).await
    }

    async fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        self.gitea.replace_pr_labels(req).await
    }

    async fn create_release(
        &self,
        tag: &str,
        sha: &str,
        notes: &str,
    ) -> Result<()> {
        self.gitea.create_release(tag, sha, notes).await
    }
}
