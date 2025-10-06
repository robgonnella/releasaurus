//! Implements the Forge trait for Gitlab
use async_trait::async_trait;
use chrono::DateTime;
use color_eyre::eyre::{ContextCompat, eyre};
use gitlab::{
    AsyncGitlab,
    api::{
        AsyncQuery, Pagination, ignore,
        merge_requests::MergeRequestState,
        paged,
        projects::{
            Project,
            labels::{CreateLabel, Labels},
            merge_requests::{
                CreateMergeRequest, EditMergeRequest, MergeRequests,
            },
            releases::CreateRelease,
            repository::{
                commits::{
                    CommitAction, CommitActionType, Commits, CommitsOrder,
                    CreateCommit,
                },
                files::FileRaw,
                tags::{CreateTag, Tags, TagsOrderBy},
            },
        },
    },
};
use log::*;
use regex::Regex;
use reqwest::StatusCode;
use secrecy::ExposeSecret;
use serde::Deserialize;
use std::{cmp, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    analyzer::release::Tag,
    config::{Config, DEFAULT_CONFIG_FILE},
    forge::{
        config::{
            DEFAULT_COMMIT_SEARCH_DEPTH, DEFAULT_LABEL_COLOR, PENDING_LABEL,
            RemoteConfig,
        },
        request::{
            Commit, CreateBranchRequest, CreatePrRequest, FileUpdateType,
            ForgeCommit, GetPrRequest, PrLabelsRequest, PullRequest,
            UpdatePrRequest,
        },
        traits::{FileLoader, Forge},
    },
    result::Result,
};

#[derive(Debug, Deserialize)]
struct MergeRequestInfo {
    iid: u64,
    sha: String,
    merged_at: Option<String>,
    merge_commit_sha: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LabelInfo {
    name: String,
}

/// Information about a commit associated with a release.
#[derive(Debug, Deserialize)]
pub struct GitlabCommit {
    pub id: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub parent_ids: Vec<String>,
    pub created_at: String,
}

/// Represents a Gitlab project Tag
#[derive(Debug, Deserialize)]
pub struct GitlabTag {
    pub name: String,
    pub commit: GitlabCommit,
}

#[derive(Debug, Deserialize)]
pub struct CreatedCommit {
    pub id: String,
}

/// GitLab forge implementation using gitlab crate for API interactions with
/// commit history, tags, merge requests, and releases.
pub struct Gitlab {
    config: RemoteConfig,
    commit_search_depth: Arc<Mutex<u64>>,
    gl: AsyncGitlab,
    project_id: String,
}

impl Gitlab {
    /// Create GitLab client with personal access token authentication and
    /// project ID resolution.
    pub async fn new(config: RemoteConfig) -> Result<Self> {
        let project_id = config.path.clone();

        let token = config.token.expose_secret();

        let gl = gitlab::GitlabBuilder::new(config.host.clone(), token)
            .build_async()
            .await?;

        Ok(Self {
            config,
            commit_search_depth: Arc::new(Mutex::new(
                DEFAULT_COMMIT_SEARCH_DEPTH,
            )),
            gl,
            project_id,
        })
    }

    /// Fetch all labels currently defined in the GitLab repository.
    async fn get_repo_labels(&self) -> Result<Vec<LabelInfo>> {
        let endpoint = Labels::builder().project(&self.project_id).build()?;

        let labels: Vec<LabelInfo> = endpoint.query_async(&self.gl).await?;

        Ok(labels)
    }

    /// Create a new label in the GitLab repository with default color.
    async fn create_label(&self, label_name: String) -> Result<LabelInfo> {
        let endpoint = CreateLabel::builder()
            .project(&self.project_id)
            .name(label_name)
            .color(format!("#{}", DEFAULT_LABEL_COLOR))
            .description("".to_string())
            .build()?;

        let label: LabelInfo = endpoint.query_async(&self.gl).await?;

        Ok(label)
    }
}

#[async_trait]
impl FileLoader for Gitlab {
    async fn get_file_content(&self, path: &str) -> Result<Option<String>> {
        let endpoint = FileRaw::builder()
            .project(&self.project_id)
            .file_path(path)
            .build()?;

        let result: std::result::Result<
            String,
            gitlab::api::ApiError<gitlab::RestError>,
        > = endpoint.query_async(&self.gl).await;

        match result {
            Err(gitlab::api::ApiError::GitlabService { status, data }) => {
                if status == StatusCode::NOT_FOUND {
                    info!("no file found for path: {path}");
                    return Ok(None);
                }
                // For some reason successful responses are returned in Err
                // ¯\_(ツ)_/¯
                if status == StatusCode::OK {
                    info!("found file at path: {path}");
                    let content = String::from_utf8(data)?;
                    return Ok(Some(content));
                }
                let msg = format!(
                    "failed to file content from repo: status: {status}, data: {}",
                    String::from_utf8(data).unwrap()
                );
                error!("{msg}");
                Err(eyre!(msg))
            }
            Err(gitlab::api::ApiError::GitlabWithStatus { status, msg }) => {
                if status == StatusCode::NOT_FOUND {
                    info!("no file found for path: {path}");
                    return Ok(None);
                }
                let msg = format!(
                    "failed to file content from repo: status: {status}, msg: {}",
                    msg
                );
                error!("{msg}");
                Err(eyre!(msg))
            }
            Err(err) => {
                error!("failed to get file from repo: {err}");
                Err(eyre!("failed to get file from repo: {err}"))
            }
            _ => {
                let msg = "unknown error occurred getting file from repo";
                error!("{msg}");
                Err(eyre!(msg))
            }
        }
    }
}

#[async_trait]
impl Forge for Gitlab {
    fn repo_name(&self) -> String {
        self.config.repo.clone()
    }

    fn remote_config(&self) -> RemoteConfig {
        self.config.clone()
    }

    async fn load_config(&self) -> Result<Config> {
        let content = self.get_file_content(DEFAULT_CONFIG_FILE).await?;

        if content.is_none() {
            info!("repository configuration not found: using default");
            return Ok(Config::default());
        }

        let content = content.unwrap();

        let config: Config = toml::from_str(&content)?;

        let mut config_search_depth = config.first_release_search_depth;
        if config_search_depth == 0 {
            config_search_depth = u64::MAX;
        }

        let mut search_depth = self.commit_search_depth.lock().await;
        *search_depth = config_search_depth;

        Ok(config)
    }

    async fn default_branch(&self) -> Result<String> {
        let endpoint = Project::builder().project(&self.project_id).build()?;
        let result: serde_json::Value = endpoint.query_async(&self.gl).await?;
        let default_branch = result["default_branch"]
            .as_str()
            .wrap_err("failed to find default branch")?;
        Ok(default_branch.to_string())
    }

    /// Get the latest release for the project
    async fn get_latest_tag_for_prefix(
        &self,
        prefix: &str,
    ) -> Result<Option<Tag>> {
        let re = Regex::new(format!(r"^{prefix}").as_str())?;
        let endpoint = Tags::builder()
            .project(&self.project_id)
            .order_by(TagsOrderBy::Updated)
            .build()?;
        let tags: Vec<GitlabTag> = endpoint.query_async(&self.gl).await?;
        for t in tags.into_iter() {
            if re.is_match(&t.name) {
                let stripped = re.replace_all(&t.name, "").to_string();
                if let Ok(sver) = semver::Version::parse(&stripped) {
                    return Ok(Some(Tag {
                        name: t.name,
                        semver: sver,
                        sha: t.commit.id,
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
        let search_depth = self.commit_search_depth.lock().await;

        let mut builder = Commits::builder();

        builder
            .project(&self.project_id)
            .path(path)
            .order(CommitsOrder::Default);

        if let Some(sha) = sha.clone() {
            let range = format!("{sha}..HEAD");
            builder.ref_name(range);
        }

        let endpoint = builder.build()?;
        let page_limit = cmp::min(100, *search_depth) as usize;
        let search_depth = *search_depth as usize;

        let result: Vec<GitlabCommit> = if sha.is_none() {
            paged(endpoint, Pagination::Limit(search_depth))
                .query_async(&self.gl)
                .await?
        } else {
            paged(endpoint, Pagination::AllPerPageLimit(page_limit))
                .query_async(&self.gl)
                .await?
        };

        let forge_commits = result
            .iter()
            .map(|c| ForgeCommit {
                author_email: c.author_email.clone(),
                author_name: c.author_name.clone(),
                id: c.id.clone(),
                link: format!("{}/{}", self.config.commit_link_base_url, c.id),
                merge_commit: c.parent_ids.len() > 1,
                message: c.message.clone().trim().into(),
                timestamp: DateTime::parse_from_rfc3339(&c.created_at)
                    .unwrap()
                    .timestamp(),
            })
            .collect::<Vec<ForgeCommit>>();

        Ok(forge_commits)
    }

    async fn create_release_branch(
        &self,
        req: CreateBranchRequest,
    ) -> Result<Commit> {
        let default_branch_name = self.default_branch().await?;

        let mut actions: Vec<CommitAction> = vec![];

        for change in req.file_changes {
            let mut content = change.content;

            let mut update_type = CommitActionType::Update;

            let existing_content = self.get_file_content(&change.path).await?;

            if existing_content.is_none() {
                update_type = CommitActionType::Create;
            }

            if matches!(change.update_type, FileUpdateType::Prepend)
                && let Some(existing_content) = existing_content
            {
                content = format!("{content}{existing_content}");
            }

            let action = CommitAction::builder()
                .action(update_type)
                .content(content.as_bytes().to_owned())
                .file_path(change.path.clone())
                .build()?;

            actions.push(action)
        }

        let endpoint = CreateCommit::builder()
            .project(&self.project_id)
            .start_branch(default_branch_name)
            .branch(req.branch)
            .actions(actions)
            .commit_message(req.message)
            .force(true)
            .build()?;

        let commit: CreatedCommit = endpoint.query_async(&self.gl).await?;

        Ok(Commit { sha: commit.id })
    }

    async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()> {
        let endpoint = CreateTag::builder()
            .project(&self.project_id)
            .message(tag_name)
            .tag_name(tag_name)
            .ref_(sha)
            .build()?;

        ignore(endpoint).query_async(&self.gl).await?;

        Ok(())
    }

    async fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        // Create the merge requests query to find open MRs
        // targeting the base branch
        let endpoint = MergeRequests::builder()
            .project(&self.project_id)
            .state(MergeRequestState::Opened)
            .source_branch(&req.head_branch)
            .target_branch(&req.base_branch)
            .build()?;

        // Execute the query to get matching merge requests
        let result: std::result::Result<
            Vec<MergeRequestInfo>,
            gitlab::api::ApiError<gitlab::RestError>,
        > = endpoint.query_async(&self.gl).await;

        // Execute the query to get matching merge requests
        match result {
            Ok(merge_requests) => {
                // Return the first matching merge request's IID
                // (should only be one for a given branch)
                let first = merge_requests.first();

                if first.is_none() {
                    return Ok(None);
                }

                let merge_request = first.unwrap();

                Ok(Some(PullRequest {
                    number: merge_request.iid,
                    sha: merge_request.sha.clone(),
                }))
            }
            Err(gitlab::api::ApiError::GitlabWithStatus { status, msg }) => {
                if status == reqwest::StatusCode::NOT_FOUND {
                    Ok(None)
                } else {
                    let msg = format!(
                        "request for pull request failed: status {status}, msg: {msg}"
                    );
                    error!("{msg}");
                    Err(eyre!(msg))
                }
            }
            Err(err) => Err(eyre!(
                "encountered error querying gitlab for merge request: {err}"
            )),
        }
    }

    async fn get_merged_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        info!("looking for closed release prs with pending label");

        // Search for closed merge requests with the pending label
        let endpoint = MergeRequests::builder()
            .project(&self.project_id)
            .state(MergeRequestState::Merged)
            .source_branch(req.head_branch.clone())
            .labels(vec![PENDING_LABEL])
            .build()?;

        let merge_requests: Vec<MergeRequestInfo> =
            endpoint.query_async(&self.gl).await?;

        if merge_requests.is_empty() {
            warn!(
                "No merged release PRs with the label {PENDING_LABEL} found for branch {}. Nothing to release",
                req.head_branch
            );
            return Ok(None);
        }

        if merge_requests.len() > 1 {
            return Err(eyre!(
                "Found more than one closed release PR with pending label for branch {}. \
                This means either release PRs were closed manually or releasaurus failed to remove tags. \
                You must remove the {PENDING_LABEL} label from all closed release PRs except for the most recent.",
                req.head_branch
            ));
        }

        let merge_request = &merge_requests[0];
        info!("found release pr: {}", merge_request.iid);

        // Check if the MR is actually merged (has merged_at timestamp)
        if merge_request.merged_at.is_none() {
            return Err(eyre!(
                "found release PR {} but it hasn't been merged yet",
                merge_request.iid
            ));
        }

        let sha = merge_request
            .merge_commit_sha
            .as_ref()
            .ok_or_else(|| eyre!("no merge_commit_sha found for pr"))?
            .clone();

        Ok(Some(PullRequest {
            number: merge_request.iid,
            sha,
        }))
    }

    async fn create_pr(&self, req: CreatePrRequest) -> Result<PullRequest> {
        // Create the merge request
        let endpoint = CreateMergeRequest::builder()
            .project(&self.project_id)
            .source_branch(&req.head_branch)
            .target_branch(&req.base_branch)
            .title(&req.title)
            .description(&req.body)
            .build()?;

        // Execute the creation
        let merge_request: MergeRequestInfo =
            endpoint.query_async(&self.gl).await?;

        Ok(PullRequest {
            number: merge_request.iid,
            sha: merge_request.sha.clone(),
        })
    }

    async fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        // Update the merge request
        let endpoint = EditMergeRequest::builder()
            .project(&self.project_id)
            .merge_request(req.pr_number)
            .title(&req.title)
            .description(&req.body)
            .build()?;

        // Execute the update using ignore since we don't need the response
        ignore(endpoint).query_async(&self.gl).await?;

        Ok(())
    }

    async fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        let all_labels = self.get_repo_labels().await?;

        let mut labels = vec![];

        for name in req.labels {
            if let Some(label) = all_labels.iter().find(|l| l.name == name) {
                labels.push(label.name.clone());
            } else {
                let label = self.create_label(name).await?;
                labels.push(label.name);
            }
        }

        // Update the merge request with combined labels
        let endpoint = EditMergeRequest::builder()
            .project(&self.project_id)
            .merge_request(req.pr_number)
            .labels(labels.iter())
            .build()?;

        // Execute the update
        ignore(endpoint).query_async(&self.gl).await?;

        Ok(())
    }

    async fn create_release(
        &self,
        tag: &str,
        sha: &str,
        notes: &str,
    ) -> Result<()> {
        // Create the release
        let endpoint = CreateRelease::builder()
            .project(&self.project_id)
            .tag_name(tag)
            .name(tag)
            .description(notes)
            .ref_sha(sha)
            .build()?;

        // Execute the creation using ignore since we don't need the response
        ignore(endpoint).query_async(&self.gl).await?;

        Ok(())
    }
}
