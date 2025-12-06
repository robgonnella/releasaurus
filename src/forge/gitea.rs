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
use std::{cmp, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};

use crate::{
    Result,
    analyzer::release::Tag,
    config::{Config, DEFAULT_CONFIG_FILE},
    forge::{
        config::{
            DEFAULT_COMMIT_SEARCH_DEPTH, DEFAULT_LABEL_COLOR,
            DEFAULT_PAGE_SIZE, PENDING_LABEL, RemoteConfig,
        },
        request::{
            Commit, CreateBranchRequest, CreatePrRequest, FileUpdateType,
            ForgeCommit, GetPrRequest, PrLabelsRequest, PullRequest,
            UpdatePrRequest,
        },
        traits::Forge,
    },
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
struct PullRequestBranch {
    label: String,
    sha: String,
}

#[derive(Debug, Deserialize)]
struct GiteaPullRequest {
    number: u64,
    head: PullRequestBranch,
    merge_commit_sha: Option<String>,
    body: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GiteaIssuePr {
    merged: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct GiteaIssue {
    number: u64,
    pull_request: GiteaIssuePr,
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
    pub html_url: String,
}

#[derive(Debug, Deserialize)]
struct GiteaTagCommit {
    pub created: String,
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

/// Gitea forge implementation using reqwest for API interactions with
/// commit history, tags, pull requests, and releases.
pub struct Gitea {
    config: RemoteConfig,
    commit_search_depth: Arc<Mutex<u64>>,
    base_url: Url,
    client: Client,
    default_branch: String,
}

impl Gitea {
    /// Create Gitea client with token authentication and API base URL
    /// configuration for self-hosted instances.
    pub async fn new(config: RemoteConfig) -> Result<Self> {
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

        let request = client.get(base_url.clone()).build()?;
        let response = client.execute(request).await?;
        let result = response.error_for_status()?;
        let repo: serde_json::Value = result.json().await?;
        let default_branch = repo["default_branch"]
            .as_str()
            .wrap_err("failed to get default branch")?;

        Ok(Self {
            config,
            commit_search_depth: Arc::new(Mutex::new(
                DEFAULT_COMMIT_SEARCH_DEPTH,
            )),
            client,
            base_url,
            default_branch: default_branch.into(),
        })
    }

    // TODO: Right now gitea does not support force updating a branch
    // Once the below issue is resolved we can remove this method and use the
    // "force" option
    // https://github.com/go-gitea/gitea/issues/35538
    async fn delete_branch_if_exists(&self, branch: &str) -> Result<()> {
        let url = self.base_url.join(&format!("branches/{branch}"))?;
        let request = self.client.delete(url).build()?;
        self.client.execute(request).await?;
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
        if self.config.dry_run {
            warn!("dry_run: would create label: {label_name}");
            return Ok(Label::default());
        }
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
impl Forge for Gitea {
    fn repo_name(&self) -> String {
        self.config.repo.clone()
    }

    fn remote_config(&self) -> RemoteConfig {
        self.config.clone()
    }

    fn default_branch(&self) -> String {
        self.default_branch.clone()
    }

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

    async fn load_config(&self) -> Result<Config> {
        if let Some(content) =
            self.get_file_content(DEFAULT_CONFIG_FILE).await?
        {
            let config: Config = toml::from_str(&content)?;

            let mut config_search_depth = config.first_release_search_depth;
            if config_search_depth == 0 {
                config_search_depth = u64::MAX;
            }

            let mut search_depth = self.commit_search_depth.lock().await;
            *search_depth = config_search_depth;

            Ok(config)
        } else {
            info!("configuration not found in repo: using default");
            Ok(Config::default())
        }
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
                        timestamp: DateTime::parse_from_rfc3339(
                            &tag.commit.created,
                        )
                        .map(|t| t.timestamp())
                        .ok(),
                    }));
                }
            }
        }

        Ok(None)
    }

    async fn get_commits(
        &self,
        sha: Option<String>,
    ) -> Result<Vec<ForgeCommit>> {
        let mut page = 1;
        let search_depth = self.commit_search_depth.lock().await;
        let page_limit = cmp::min(DEFAULT_PAGE_SIZE.into(), *search_depth);
        let mut has_more = true;
        let mut count = 0;
        let mut commits: Vec<ForgeCommit> = vec![];

        let mut since = None;

        if let Some(sha) = sha.clone() {
            let commit_url = self.base_url.join(&format!("commits/{sha}"))?;
            let request = self.client.get(commit_url).build()?;
            let response = self.client.execute(request).await?;
            let result = response.error_for_status()?;
            let commit: GiteaCommitQueryObject = result.json().await?;
            since = Some(commit.created);
        }

        while has_more {
            let mut commits_url = self.base_url.join("commits")?;

            commits_url
                .query_pairs_mut()
                .append_pair("limit", &page_limit.to_string())
                .append_pair("page", &page.to_string());

            if let Some(since) = since.clone() {
                commits_url.query_pairs_mut().append_pair("since", &since);
            }

            let request = self.client.get(commits_url).build()?;
            let response = self.client.execute(request).await?;
            let headers = response.headers();

            has_more = headers
                .get("x-hasmore")
                .map(|h| h.to_str().unwrap() == "true")
                .unwrap_or(false);

            let result = response.error_for_status()?;
            let results: Vec<GiteaCommitQueryObject> = result.json().await?;

            for result in results.iter() {
                // only apply search depth if this is the first release
                if sha.is_none() && count >= *search_depth {
                    return Ok(commits);
                }

                // we've reached the target sha stopping point
                // this is because "since" is inclusive of the target commit
                if let Some(sha) = sha.clone()
                    && sha == result.sha
                {
                    return Ok(commits);
                }

                let timestamp =
                    DateTime::parse_from_rfc3339(&result.created)?.timestamp();

                let forge_commit = ForgeCommit {
                    author_email: result.commit.author.email.clone(),
                    author_name: result.commit.author.name.clone(),
                    id: result.sha.clone(),
                    short_id: result
                        .sha
                        .clone()
                        .split("")
                        .take(8)
                        .collect::<Vec<&str>>()
                        .join(""),
                    link: result.html_url.clone(),
                    merge_commit: result.parents.len() > 1,
                    message: result.commit.message.trim().to_string(),
                    timestamp,
                    files: result
                        .files
                        .iter()
                        .map(|f| f.filename.clone())
                        .collect::<Vec<String>>(),
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
        // pause execution to wait for any PRs that might have been closed as
        // a result to fully register as closed
        sleep(Duration::from_millis(3000)).await;

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
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        info!("looking for open release prs with pending label");

        // Search for open issues with the pending label
        let issues_url = self.base_url.join(&format!(
            "issues?state=open&type=pulls&labels={}",
            PENDING_LABEL
        ))?;

        let request = self.client.get(issues_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let issues: Vec<GiteaIssue> = result.json().await?;

        if issues.is_empty() {
            return Ok(None);
        }

        let mut pr = None;

        for issue in issues.iter() {
            let pr_url =
                self.base_url.join(&format!("pulls/{}", issue.number))?;
            let request = self.client.get(pr_url).build()?;
            let response = self.client.execute(request).await?;
            let result = response.error_for_status()?;
            let found_pr: GiteaPullRequest = result.json().await?;
            if found_pr.head.label == req.head_branch {
                pr = Some(found_pr);
                break;
            }
        }

        if let Some(pr) = pr {
            info!(
                "found open release pr: {} for branch {}",
                pr.number, req.head_branch
            );

            let sha = pr.head.sha;

            Ok(Some(PullRequest {
                number: pr.number,
                sha,
                body: pr.body,
            }))
        } else {
            warn!("No open release PRs found for branch {}", req.head_branch);
            Ok(None)
        }
    }

    async fn get_merged_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        info!("looking for closed release prs with pending label for branch");

        // Search for closed issues with the pending label
        let issues_url = self
            .base_url
            .join(&format!("issues?state=closed&labels={} ", PENDING_LABEL))?;

        let request = self.client.get(issues_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let issues: Vec<GiteaIssue> = result.json().await?;

        if issues.is_empty() {
            warn!(
                "No merged release PRs with the label {PENDING_LABEL} found for branch {}",
                req.head_branch,
            );
            return Ok(None);
        }

        let mut pr = None;

        for issue in issues.iter() {
            if !issue.pull_request.merged {
                warn!(
                    "found unmerged closed pr {} with pending label: skipping",
                    issue.number
                );
                continue;
            }

            let pr_url =
                self.base_url.join(&format!("pulls/{}", issue.number))?;
            let request = self.client.get(pr_url).build()?;
            let response = self.client.execute(request).await?;
            let result = response.error_for_status()?;
            let found_pr: GiteaPullRequest = result.json().await?;
            if found_pr.head.label == req.head_branch {
                pr = Some(found_pr);
                break;
            }
        }

        if let Some(pr) = pr {
            info!("found merged release pr: {}", pr.number);

            let sha = pr.merge_commit_sha.ok_or_else(|| {
                eyre!("no merge_commit_sha found for pr {}", pr.number)
            })?;

            Ok(Some(PullRequest {
                number: pr.number,
                sha,
                body: pr.body,
            }))
        } else {
            warn!(
                "No merged release PRs with the label {PENDING_LABEL} found for branch {}",
                req.head_branch,
            );
            Ok(None)
        }
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
            body: pr.body,
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
