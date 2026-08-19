use async_trait::async_trait;
use base64::{Engine, prelude::BASE64_STANDARD};
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::{
    forge::{
        config::{RepoUrl, TokenVar, USER_AGENT, resolve_token},
        forgejo::types::{
            ForgejoCreatedCommit, ForgejoFileChange,
            ForgejoFileChangeOperation, ForgejoModifyFiles,
        },
        gitea::Gitea,
        request::{
            Commit, CreatePrRequest, ForgeCommit, ForgeCommitPR,
            GetFileContentRequest, GetPrRequest, PrLabelsRequest, PullRequest,
            ReleaseByTagResponse, ResolvedCreateCommitRequest,
            ResolvedCreateReleaseBranchRequest, ResolvedFileChangeAction, Tag,
            TagResponse, UpdatePrRequest,
        },
        traits::Forge,
    },
    result::Result,
};

mod types;

pub struct Forgejo {
    gitea: Gitea,
    base_url: Url,
    client: Client,
}

impl Forgejo {
    pub async fn new(
        url: RepoUrl,
        token: Option<SecretString>,
    ) -> Result<Self> {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .ok();

        let token = resolve_token(
            token,
            url.token.as_ref(),
            vec![TokenVar::ReleasaurusForgejo, TokenVar::Forgejo],
        )?;

        let mut headers = HeaderMap::new();

        let token_value = HeaderValue::from_str(
            format!("token {}", token.expose_secret()).as_str(),
        )?;

        headers.append("Authorization", token_value);
        headers
            .append("User-Agent", HeaderValue::from_str(USER_AGENT.as_str())?);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        let base_url = match url.port {
            Some(port) => format!(
                "{}://{}:{}/api/v1/repos/{}/{}/",
                url.scheme, url.host, port, url.owner, url.name
            ),
            None => format!(
                "{}://{}/api/v1/repos/{}/{}/",
                url.scheme, url.host, url.owner, url.name
            ),
        };

        let base_url = Url::parse(&base_url)?;

        let gitea = Gitea::new(
            url.clone(),
            Some(token),
            Some(vec![TokenVar::ReleasaurusForgejo, TokenVar::Forgejo]),
        )
        .await?;

        Ok(Self {
            client,
            base_url,
            gitea,
        })
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
        starting_sha: Option<String>,
    ) -> Result<Vec<Tag>> {
        self.gitea
            .get_latest_tags_for_prefix(prefix, branch, starting_sha)
            .await
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
        req: ResolvedCreateReleaseBranchRequest,
    ) -> Result<Commit> {
        let mut file_changes: Vec<ForgejoFileChange> = vec![];

        for change in req.file_changes.iter() {
            let mut op = ForgejoFileChangeOperation::Update;
            let mut sha = None;
            let content = change.full_content.clone();
            if matches!(change.action, ResolvedFileChangeAction::Update) {
                sha = Some(
                    self.gitea
                        .get_file_sha(&req.base_branch, &change.repo_path)
                        .await?,
                );
            } else {
                op = ForgejoFileChangeOperation::Create;
            }

            file_changes.push(ForgejoFileChange {
                path: change.repo_path.clone(),
                content: BASE64_STANDARD.encode(&content),
                operation: op,
                sha,
            })
        }

        let body = ForgejoModifyFiles {
            branch: req.base_branch,
            new_branch: Some(req.release_branch),
            message: req.message,
            files: file_changes,
            force_overwrite_new_branch: Some(true),
        };

        let contents_url = self.base_url.join("contents")?;
        let request = self.client.post(contents_url).json(&body).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let created: ForgejoCreatedCommit = result.json().await?;

        Ok(created.commit)
    }

    async fn get_merged_pull_request_for_commit(
        &self,
        commit_sha: &str,
        branch: Option<String>,
    ) -> Result<Option<ForgeCommitPR>> {
        self.gitea
            .get_merged_pull_request_for_commit(commit_sha, branch)
            .await
    }

    async fn create_commit(
        &self,
        req: ResolvedCreateCommitRequest,
    ) -> Result<Commit> {
        let mut file_changes: Vec<ForgejoFileChange> = vec![];

        for change in req.file_changes.iter() {
            let mut op = ForgejoFileChangeOperation::Update;
            let mut sha = None;
            let content = change.full_content.clone();
            if matches!(change.action, ResolvedFileChangeAction::Update) {
                sha = Some(
                    self.gitea
                        .get_file_sha(&req.target_branch, &change.repo_path)
                        .await?,
                );
            } else {
                op = ForgejoFileChangeOperation::Create;
            }

            file_changes.push(ForgejoFileChange {
                path: change.repo_path.clone(),
                content: BASE64_STANDARD.encode(&content),
                operation: op,
                sha,
            })
        }

        let body = ForgejoModifyFiles {
            new_branch: None,
            branch: req.target_branch,
            message: req.message,
            files: file_changes,
            force_overwrite_new_branch: None,
        };

        let contents_url = self.base_url.join("contents")?;
        let request = self.client.post(contents_url).json(&body).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let created: ForgejoCreatedCommit = result.json().await?;

        Ok(created.commit)
    }

    async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()> {
        self.gitea.tag_commit(tag_name, sha).await
    }

    async fn get_tag(&self, tag_name: &str) -> Result<Option<TagResponse>> {
        self.gitea.get_tag(tag_name).await
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
