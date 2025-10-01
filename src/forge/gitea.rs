//! Implements the Forge trait for Gitea
use async_trait::async_trait;
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::DateTime;
use color_eyre::eyre::{ContextCompat, eyre};
use log::*;
use regex::Regex;
use reqwest::{
    Client, StatusCode, Url,
    header::{HeaderMap, HeaderValue},
};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::{cmp, path::Path};

use crate::{
    analyzer::release::Tag,
    config::{Config, DEFAULT_CONFIG_FILE},
    forge::{
        config::{DEFAULT_LABEL_COLOR, PENDING_LABEL, RemoteConfig},
        request::{
            Commit, CreateBranchRequest, CreatePrRequest, FileUpdateType,
            ForgeCommit, GetPrRequest, PrLabelsRequest, PullRequest,
            UpdatePrRequest,
        },
        traits::{FileLoader, Forge},
    },
    result::Result,
};

#[derive(Debug, Default, Serialize)]
struct CreateLabel {
    name: String,
    color: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: u64,
    pub name: String,
    color: String,
    description: String,
    exclusive: bool,
    is_archived: bool,
}

#[derive(Debug, Deserialize)]
struct PullRequestHead {
    sha: String,
}

#[derive(Debug, Deserialize)]
struct GiteaPullRequest {
    number: u64,
    head: PullRequestHead,
    merged: Option<bool>,
    merge_commit_sha: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Issue {
    number: u64,
}

#[derive(Debug, Serialize)]
struct CreatePull {
    title: String,
    body: String,
    head: String,
    base: String,
}

#[derive(Debug, Serialize)]
struct UpdatePullBody {
    title: String,
    body: String,
}

#[derive(Debug, Serialize)]
struct UpdatePullLabels {
    labels: Vec<u64>,
}

#[derive(Debug, Serialize)]
struct CreateRelease {
    tag_name: String,
    target_commitish: String,
    name: String,
    body: String,
    draft: bool,
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct CommitAuthor {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
struct GiteaCommitParent {}

#[derive(Debug, Deserialize)]
struct GiteaCommit {
    pub author: CommitAuthor,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct GiteaCommitFile {
    pub filename: String,
}

#[derive(Debug, Deserialize)]
struct GiteaCommitQueryObject {
    pub sha: String,
    pub created: String,
    pub commit: GiteaCommit,
    pub files: Vec<GiteaCommitFile>,
    pub parents: Vec<GiteaCommitParent>,
}

#[derive(Debug, Deserialize)]
struct GiteaTagCommit {
    pub sha: String,
}

#[derive(Debug, Deserialize)]
struct GiteaTag {
    name: String,
    commit: GiteaTagCommit,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum GiteaFileChangeOperation {
    Create,
    Update,
}

#[derive(Debug, Serialize)]
struct GiteaFileChange {
    pub path: String,
    pub content: String,
    pub operation: GiteaFileChangeOperation,
    pub sha: Option<String>,
}

#[derive(Debug, Serialize)]
// TODO: Currently gitea does not support the force option
// Update once below issue is resolved
// https://github.com/go-gitea/gitea/issues/35538
struct GiteaModifyFiles {
    pub new_branch: String,
    pub message: String,
    pub files: Vec<GiteaFileChange>,
    // pub force: bool,
}

#[derive(Debug, Deserialize)]
struct GiteaCreatedCommit {
    pub commit: Commit,
}

pub struct Gitea {
    config: RemoteConfig,
    base_url: Url,
    client: Client,
}

impl Gitea {
    pub fn new(config: RemoteConfig) -> Result<Self> {
        let token = config.token.expose_secret();

        let mut headers = HeaderMap::new();

        let token_value =
            HeaderValue::from_str(format!("token {}", token).as_str())?;

        headers.append("Authorization", token_value);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        let mut base_url = format!(
            "{}://{}/api/v1/repos/{}/",
            config.scheme, config.host, config.path
        );

        if let Some(port) = config.port {
            base_url = format!(
                "{}://{}:{}/api/v1/repos/{}/",
                config.scheme, config.host, port, config.path
            );
        }

        let base_url = Url::parse(&base_url)?;

        Ok(Gitea {
            config,
            client,
            base_url,
        })
    }

    // TODO: Right now gitea does not support force updating a branch
    // Once the below issue is resolved we can remove this method and use the
    // "force" option
    // https://github.com/go-gitea/gitea/issues/35538
    async fn delete_branch_if_exists(&self, branch: &str) -> Result<()> {
        let url = self.base_url.join(&format!("branches/{branch}"))?;
        let request = self.client.delete(url).build()?;
        let resp = self.client.execute(request).await?;
        info!("delete branch resp --> {:#?}", resp);
        Ok(())
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

    async fn get_all_labels(&self) -> Result<Vec<Label>> {
        let labels_url = self.base_url.join("labels")?;
        let request = self.client.get(labels_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let labels: Vec<Label> = result.json().await?;
        Ok(labels)
    }

    async fn create_label(&self, label_name: String) -> Result<Label> {
        let labels_url = self.base_url.join("labels")?;
        let request = self
            .client
            .post(labels_url)
            .json(&CreateLabel {
                name: label_name,
                color: DEFAULT_LABEL_COLOR.to_string(),
            })
            .build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let label: Label = result.json().await?;
        Ok(label)
    }
}

#[async_trait]
impl FileLoader for Gitea {
    async fn get_file_content(&self, path: &str) -> Result<Option<String>> {
        let raw_url = self.base_url.join(&format!("raw/{path}"))?;
        let request = self.client.get(raw_url).build()?;
        let response = self.client.execute(request).await?;
        if response.status() == StatusCode::NOT_FOUND {
            info!("no file found in repo at path: {path}");
            return Ok(None);
        }
        let result = response.error_for_status()?;
        let content = result.text().await?;
        Ok(Some(content))
    }
}

#[async_trait]
impl Forge for Gitea {
    fn repo_name(&self) -> String {
        self.config.repo.clone()
    }

    async fn load_config(&self) -> Result<Config> {
        let content = self.get_file_content(DEFAULT_CONFIG_FILE).await?;

        if content.is_none() {
            info!("configuration not found in repo: using default");
            return Ok(Config::default());
        }

        let content = content.unwrap();
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    async fn default_branch(&self) -> Result<String> {
        let request = self.client.get(self.base_url.clone()).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let repo: serde_json::Value = result.json().await?;
        let branch = repo["default_branch"]
            .as_str()
            .wrap_err("failed to get default branch")?;
        Ok(branch.into())
    }

    async fn get_latest_tag_for_prefix(
        &self,
        prefix: &str,
    ) -> Result<Option<Tag>> {
        let re = Regex::new(format!(r"^{prefix}").as_str())?;

        // Search for open issues with the pending label
        let tags_url = self.base_url.join("tags")?;
        let request = self.client.get(tags_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let tags: Vec<GiteaTag> = result.json().await?;

        for tag in tags.into_iter() {
            if re.is_match(&tag.name) {
                let stripped = re.replace_all(&tag.name, "").to_string();
                if let Ok(sver) = semver::Version::parse(&stripped) {
                    return Ok(Some(Tag {
                        name: tag.name,
                        semver: sver,
                        sha: tag.commit.sha,
                    }));
                }
            }
        }

        Ok(None)
    }

    async fn get_commits(
        &self,
        path: &str,
        sha: Option<String>,
    ) -> Result<Vec<ForgeCommit>> {
        let mut page = 1;
        let page_limit = cmp::min(100, self.config.commit_search_depth);
        let mut has_more = true;
        let mut count = 0;
        let mut commits: Vec<ForgeCommit> = vec![];

        while has_more {
            let mut commits_url = self.base_url.join("commits")?;

            commits_url
                .query_pairs_mut()
                .append_pair("limit", &page_limit.to_string())
                .append_pair("page", &page.to_string());

            let request = self.client.get(commits_url).build()?;
            let response = self.client.execute(request).await?;
            let headers = response.headers();
            has_more = headers
                .get("x-hasmore")
                .map(|h| h.to_str().unwrap() == "true")
                .unwrap_or(false);
            let result = response.error_for_status()?;
            let results: Vec<GiteaCommitQueryObject> = result.json().await?;

            for result in results {
                if sha.is_none() && count >= self.config.commit_search_depth {
                    return Ok(commits);
                }

                if let Some(sha) = sha.clone()
                    && sha == result.sha
                {
                    return Ok(commits);
                }

                if path != "." {
                    let p = Path::new(path);
                    let mut keep = false;

                    for file in result.files {
                        let file_path = Path::new(&file.filename);
                        if file_path.starts_with(p) {
                            keep = true;
                            break;
                        }
                    }

                    if !keep {
                        continue;
                    }
                }

                let forge_commit = ForgeCommit {
                    author_email: result.commit.author.email,
                    author_name: result.commit.author.name,
                    id: result.sha.clone(),
                    link: format!(
                        "{}/{}",
                        self.config.commit_link_base_url, result.sha
                    ),
                    merge_commit: result.parents.len() > 1,
                    message: result.commit.message.trim().to_string(),
                    timestamp: DateTime::parse_from_rfc3339(&result.created)
                        .unwrap()
                        .timestamp(),
                };

                commits.push(forge_commit);
                count += 1;
            }

            page += 1;
        }

        Ok(commits)
    }

    async fn create_release_branch(
        &self,
        req: CreateBranchRequest,
    ) -> Result<Commit> {
        // TODO: Once below issue is resolved we can delete this call
        // https://github.com/go-gitea/gitea/issues/35538
        self.delete_branch_if_exists(&req.branch).await?;

        let mut file_changes: Vec<GiteaFileChange> = vec![];

        for change in req.file_changes.iter() {
            let mut op = GiteaFileChangeOperation::Update;
            let mut sha = None;
            let mut content = change.content.clone();
            let existing_content = self.get_file_content(&change.path).await?;
            if let Some(existing_content) = existing_content {
                sha = Some(self.get_file_sha(&change.path).await?);
                if matches!(change.update_type, FileUpdateType::Prepend) {
                    content = format!("{content}{existing_content}");
                }
            } else {
                op = GiteaFileChangeOperation::Create;
            }
            file_changes.push(GiteaFileChange {
                path: change.path.clone(),
                content: BASE64_STANDARD.encode(&content),
                operation: op,
                sha,
            })
        }

        // TODO: Currently gitea does not support the force option
        // Update once below issue is resolved
        // https://github.com/go-gitea/gitea/issues/35538
        let body = GiteaModifyFiles {
            new_branch: req.branch,
            message: req.message,
            files: file_changes,
            // force: true,
        };

        let contents_url = self.base_url.join("contents")?;
        let request = self.client.post(contents_url).json(&body).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let created: GiteaCreatedCommit = result.json().await?;

        Ok(created.commit)
    }

    async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()> {
        let tag_url = self.base_url.join("tags")?;
        let body = serde_json::json!({
          "tag_name": tag_name,
          "target": sha
        });
        let request = self.client.post(tag_url).json(&body).build()?;
        let response = self.client.execute(request).await?;
        response.error_for_status()?;
        Ok(())
    }

    async fn get_open_release_pr(
        &self,
        _req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        info!("looking for open release prs with pending label");

        // Search for open issues with the pending label
        let issues_url = self.base_url.join(&format!(
            "issues?state=open&type=pulls&labels={}&page=1&limit=2",
            PENDING_LABEL
        ))?;

        let request = self.client.get(issues_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let issues: Vec<Issue> = result.json().await?;

        info!("found open PRs --> {:#?}", issues);

        if issues.is_empty() {
            return Ok(None);
        }

        if issues.len() > 1 {
            return Err(eyre!(
                "Found more than one open release PR with pending label: {}. \
                This means either releasaurus incorrectly created more than one open PR, or \
                the pending label was manually added to another PR and must be removed",
                PENDING_LABEL
            ));
        }

        let issue = &issues[0];
        info!("found open release pr: {}", issue.number);

        // Get the full PR details
        let pr_url = self.base_url.join(&format!("pulls/{}", issue.number))?;
        let request = self.client.get(pr_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let pr: GiteaPullRequest = result.json().await?;

        // Make sure the PR isn't merged
        if let Some(merged) = pr.merged
            && merged
        {
            return Err(eyre!(
                "found release PR {} but it has already been merged",
                pr.number
            ));
        }

        let sha = pr.head.sha;

        Ok(Some(PullRequest {
            number: pr.number,
            sha,
        }))
    }

    async fn get_merged_release_pr(&self) -> Result<Option<PullRequest>> {
        info!("looking for closed release prs with pending label");

        // Search for closed issues with the pending label
        let issues_url = self.base_url.join(&format!(
            "issues?state=closed&labels={}&page=1&limit=2",
            PENDING_LABEL
        ))?;

        let request = self.client.get(issues_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let issues: Vec<Issue> = result.json().await?;

        if issues.is_empty() {
            warn!(
                "No merged release PRs with the label {} found. Nothing to release",
                PENDING_LABEL
            );
            return Ok(None);
        }

        if issues.len() > 1 {
            return Err(eyre!(
                "Found more than one closed release PR with pending label. \
                This means either release PRs were closed manually or releasaurus failed to remove tags. \
                You must remove the {} label from all closed release PRs except for the most recent.",
                PENDING_LABEL
            ));
        }

        let issue = &issues[0];
        info!("found release pr: {}", issue.number);

        // Get the full PR details
        let pr_url = self.base_url.join(&format!("pulls/{}", issue.number))?;
        let request = self.client.get(pr_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let pr: GiteaPullRequest = result.json().await?;

        // Check if the PR is actually merged
        if let Some(merged) = pr.merged
            && !merged
        {
            return Err(eyre!(
                "found release PR {} but it hasn't been merged yet",
                pr.number
            ));
        }

        let sha = pr
            .merge_commit_sha
            .ok_or_else(|| eyre!("no merge_commit_sha found for pr"))?;

        Ok(Some(PullRequest {
            number: pr.number,
            sha,
        }))
    }

    async fn create_pr(&self, req: CreatePrRequest) -> Result<PullRequest> {
        let data = CreatePull {
            title: req.title,
            body: req.body,
            head: req.head_branch,
            base: req.base_branch,
        };
        let pulls_url = self.base_url.join("pulls")?;
        let request = self.client.post(pulls_url).json(&data).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let pr: GiteaPullRequest = result.json().await?;

        Ok(PullRequest {
            number: pr.number,
            sha: pr.head.sha,
        })
    }

    async fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        let data = UpdatePullBody {
            title: req.title,
            body: req.body,
        };
        let pulls_url = self
            .base_url
            .join(format!("pulls/{}", req.pr_number).as_str())?;
        let request = self.client.patch(pulls_url).json(&data).build()?;
        let response = self.client.execute(request).await?;
        response.error_for_status()?;
        Ok(())
    }

    async fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        let all_labels = self.get_all_labels().await?;

        let mut labels = vec![];

        for name in req.labels {
            if let Some(label) = all_labels.iter().find(|l| l.name == name) {
                labels.push(label.id);
            } else {
                let label = self.create_label(name).await?;
                labels.push(label.id);
            }
        }

        let data = UpdatePullLabels { labels };

        let labels_url = self
            .base_url
            .join(format!("issues/{}/labels", req.pr_number).as_str())?;

        let request = self.client.put(labels_url).json(&data).build()?;
        let response = self.client.execute(request).await?;
        response.error_for_status()?;

        Ok(())
    }

    async fn create_release(
        &self,
        tag: &str,
        sha: &str,
        notes: &str,
    ) -> Result<()> {
        let data = CreateRelease {
            tag_name: tag.to_string(),
            target_commitish: sha.to_string(),
            name: tag.to_string(),
            body: notes.to_string(),
            draft: false,
            prerelease: false,
        };

        let releases_url = self.base_url.join("releases")?;
        let request = self.client.post(releases_url).json(&data).build()?;
        let response = self.client.execute(request).await?;
        response.error_for_status()?;

        Ok(())
    }
}
