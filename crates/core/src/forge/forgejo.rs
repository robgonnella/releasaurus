use async_trait::async_trait;
use base64::{Engine, prelude::BASE64_STANDARD};
use color_eyre::eyre::ContextCompat;
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::{
    config::Config,
    forge::{
        config::{RepoUrl, TokenVar, resolve_token},
        forgejo::types::{
            ForgejoCreatedCommit, ForgejoFileChange,
            ForgejoFileChangeOperation, ForgejoModifyFiles,
        },
        gitea::Gitea,
        request::{
            Commit, CreateCommitRequest, CreatePrRequest,
            CreateReleaseBranchRequest, FileUpdateType, ForgeCommit,
            GetFileContentRequest, GetPrRequest, PrLabelsRequest, PullRequest,
            ReleaseByTagResponse, Tag, UpdatePrRequest,
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

        let token =
            resolve_token(token, url.token.as_ref(), TokenVar::Forgejo)?;

        let mut headers = HeaderMap::new();

        let token_value = HeaderValue::from_str(
            format!("token {}", token.expose_secret()).as_str(),
        )?;

        headers.append("Authorization", token_value);

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

        let gitea =
            Gitea::new(url.clone(), Some(token), Some(TokenVar::Forgejo))
                .await?;

        Ok(Self {
            client,
            base_url,
            gitea,
        })
    }

    async fn get_file_sha(&self, path: &str) -> Result<String> {
        let path = path.strip_prefix("./").unwrap_or(path);
        let file_url = self.base_url.join(&format!("contents/{path}"))?;
        let request = self.client.get(file_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let file: serde_json::Value = result.json().await?;
        let sha = file["sha"].as_str().wrap_err("failed to get file sha")?;
        Ok(sha.into())
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
        let mut file_changes: Vec<ForgejoFileChange> = vec![];

        for change in req.file_changes.iter() {
            let mut op = ForgejoFileChangeOperation::Update;
            let mut sha = None;
            let mut content = change.content.clone();
            let existing_content = self
                .get_file_content(GetFileContentRequest {
                    branch: Some(req.base_branch.clone()),
                    path: change.path.to_string(),
                })
                .await?;
            if let Some(existing_content) = existing_content {
                sha = Some(self.get_file_sha(&change.path).await?);
                if matches!(change.update_type, FileUpdateType::Prepend) {
                    content = format!("{content}{existing_content}");
                }
            } else {
                op = ForgejoFileChangeOperation::Create;
            }
            file_changes.push(ForgejoFileChange {
                path: change.path.clone(),
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
        };

        let contents_url = self.base_url.join("contents")?;
        let request = self.client.post(contents_url).json(&body).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let created: ForgejoCreatedCommit = result.json().await?;

        Ok(created.commit)
    }

    async fn create_commit(&self, req: CreateCommitRequest) -> Result<Commit> {
        let mut file_changes: Vec<ForgejoFileChange> = vec![];

        for change in req.file_changes.iter() {
            let mut op = ForgejoFileChangeOperation::Update;
            let mut sha = None;
            let mut content = change.content.clone();
            let existing_content = self
                .get_file_content(GetFileContentRequest {
                    branch: Some(req.target_branch.clone()),
                    path: change.path.to_string(),
                })
                .await?;
            if let Some(existing_content) = existing_content.clone() {
                sha = Some(self.get_file_sha(&change.path).await?);
                if matches!(change.update_type, FileUpdateType::Prepend) {
                    content = format!("{content}{existing_content}");
                }
            } else {
                op = ForgejoFileChangeOperation::Create;
            }

            if content == existing_content.unwrap_or_default() {
                log::warn!(
                    "skipping file update content matches existing state: {}",
                    change.path
                );
                continue;
            }

            file_changes.push(ForgejoFileChange {
                path: change.path.clone(),
                content: BASE64_STANDARD.encode(&content),
                operation: op,
                sha,
            })
        }

        if file_changes.is_empty() {
            log::warn!(
                "commit would result in no changes: target_branch: {}, message: {}",
                req.target_branch,
                req.message,
            );
            return Ok(Commit { sha: "None".into() });
        }

        let body = ForgejoModifyFiles {
            new_branch: None,
            branch: req.target_branch,
            message: req.message,
            files: file_changes,
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
