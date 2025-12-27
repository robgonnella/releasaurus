//! Implements the Forge trait for Gitlab
use async_trait::async_trait;
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::DateTime;
use color_eyre::eyre::ContextCompat;
use derive_builder::Builder;
use gitlab::{
    AsyncGitlab,
    api::{
        AsyncQuery, Endpoint, Pagination, QueryParams,
        common::NameOrId,
        ignore,
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
                files::File,
                tags::{CreateTag, Tag as GitlabTagBuilder, Tags, TagsOrderBy},
            },
        },
    },
};
use graphql_client::{GraphQLQuery, QueryBody};
use log::*;
use regex::Regex;
use reqwest::{Method, StatusCode};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cmp, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    Result,
    analyzer::release::Tag,
    config::{Config, DEFAULT_CONFIG_FILE},
    error::ReleasaurusError,
    forge::{
        config::{
            DEFAULT_COMMIT_SEARCH_DEPTH, DEFAULT_LABEL_COLOR,
            DEFAULT_PAGE_SIZE, PENDING_LABEL, RemoteConfig,
        },
        request::{
            Commit, CreateCommitRequest, CreatePrRequest,
            CreateReleaseBranchRequest, FileUpdateType, ForgeCommit,
            GetFileContentRequest, GetPrRequest, PrLabelsRequest, PullRequest,
            ReleaseByTagResponse, UpdatePrRequest,
        },
        traits::Forge,
    },
};

const COMMIT_DIFF_QUERY: &str = r#"
query GetCommitDiff($project_id: ID!, $commit_sha: String!) {
  project(fullPath: $project_id) {
    repository {
      commit(ref: $commit_sha) {
        diffs {
            newPath
            oldPath
        }
      }
    }
  }
}"#;

#[derive(Debug, Serialize)]
struct CommitDiffQueryVars {
    project_id: String,
    commit_sha: String,
}

#[derive(Debug, Deserialize)]
struct CommitFilenameDiff {
    #[serde(rename = "oldPath")]
    old_path: Option<String>,
    #[serde(rename = "newPath")]
    new_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CommitDiffCommit {
    diffs: Vec<CommitFilenameDiff>,
}

#[derive(Debug, Deserialize)]
struct CommitDiffRepository {
    commit: CommitDiffCommit,
}

#[derive(Debug, Deserialize)]
struct CommitDiffProject {
    repository: CommitDiffRepository,
}

#[derive(Debug, Deserialize)]
struct CommitDiffResponse {
    project: CommitDiffProject,
}

struct CommitDiffQuery {}

impl GraphQLQuery for CommitDiffQuery {
    type ResponseData = CommitDiffResponse;
    type Variables = CommitDiffQueryVars;

    fn build_query(variables: Self::Variables) -> QueryBody<Self::Variables> {
        QueryBody {
            variables,
            query: COMMIT_DIFF_QUERY,
            operation_name: "GetCommitDiff",
        }
    }
}

#[derive(Debug, Deserialize)]
struct FileInfo {
    content: String,
}

#[derive(Debug, Deserialize)]
struct MergeRequestInfo {
    iid: u64,
    merge_commit_sha: Option<String>,
    sha: String,
    merged_at: Option<String>,
    description: String,
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
    pub web_url: String,
}

/// Represents a Gitlab project Tag
#[derive(Debug, Deserialize)]
pub struct GitlabTag {
    pub name: String,
    pub commit: GitlabCommit,
}

/// Represents a Gitlab release
#[derive(Debug, Deserialize)]
pub struct GitlabRelease {
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatedCommit {
    pub id: String,
}

/// Query a specific project release by tag name.
#[derive(Debug, Builder)]
#[builder(setter(strip_option))]
pub struct ProjectReleaseByTag<'a> {
    /// The project to query for release.
    #[builder(setter(into))]
    project: NameOrId<'a>,

    /// Gets a release for a specific tag
    tag: Cow<'a, str>,
}

impl<'a> ProjectReleaseByTag<'a> {
    /// Create a builder for the endpoint.
    pub fn builder() -> ProjectReleaseByTagBuilder<'a> {
        ProjectReleaseByTagBuilder::default()
    }
}

impl Endpoint for ProjectReleaseByTag<'_> {
    fn method(&self) -> Method {
        Method::GET
    }

    fn endpoint(&self) -> Cow<'static, str> {
        format!("projects/{}/releases/{}", self.project, self.tag).into()
    }

    fn parameters(&self) -> QueryParams<'_> {
        QueryParams::default()
    }
}

/// GitLab forge implementation using gitlab crate for API interactions with
/// commit history, tags, merge requests, and releases.
pub struct Gitlab {
    config: RemoteConfig,
    commit_search_depth: Arc<Mutex<u64>>,
    gl: AsyncGitlab,
    project_id: String,
    default_branch: String,
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

        let endpoint = Project::builder().project(&project_id).build()?;
        let gl_project: serde_json::Value = endpoint.query_async(&gl).await?;

        let default_branch = gl_project["default_branch"]
            .as_str()
            .wrap_err("failed to find default branch")?
            .to_string();

        Ok(Self {
            config,
            commit_search_depth: Arc::new(Mutex::new(
                DEFAULT_COMMIT_SEARCH_DEPTH,
            )),
            gl,
            project_id,
            default_branch,
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
impl Forge for Gitlab {
    fn repo_name(&self) -> String {
        self.config.repo.clone()
    }

    fn remote_config(&self) -> RemoteConfig {
        self.config.clone()
    }

    fn default_branch(&self) -> String {
        self.default_branch.clone()
    }

    async fn load_config(&self, branch: Option<String>) -> Result<Config> {
        if let Some(content) = self
            .get_file_content(GetFileContentRequest {
                branch,
                path: DEFAULT_CONFIG_FILE.into(),
            })
            .await?
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
            info!("repository configuration not found: using default");
            Ok(Config::default())
        }
    }

    async fn get_file_content(
        &self,
        req: GetFileContentRequest,
    ) -> Result<Option<String>> {
        let r#ref = req.branch.unwrap_or("HEAD".into());

        let endpoint = File::builder()
            .project(&self.project_id)
            .file_path(&req.path)
            .ref_(&r#ref)
            .build()?;

        let result: std::result::Result<
            FileInfo,
            gitlab::api::ApiError<gitlab::RestError>,
        > = endpoint.query_async(&self.gl).await;

        match result {
            Ok(file_info) => {
                let decoded = BASE64_STANDARD.decode(file_info.content)?;
                let content = String::from_utf8(decoded)?;
                return Ok(Some(content));
            }
            Err(gitlab::api::ApiError::GitlabService { status, data }) => {
                if status == StatusCode::NOT_FOUND {
                    return Ok(None);
                }
                let msg = format!(
                    "failed to file content from repo: status: {status}, data: {}",
                    String::from_utf8(data).unwrap()
                );
                error!("{msg}");
                Err(ReleasaurusError::forge(msg))
            }
            Err(gitlab::api::ApiError::GitlabWithStatus { status, msg }) => {
                if status == StatusCode::NOT_FOUND {
                    return Ok(None);
                }
                let msg = format!(
                    "failed to file content from repo: status: {status}, msg: {}",
                    msg
                );
                error!("{msg}");
                Err(ReleasaurusError::forge(msg))
            }
            Err(err) => {
                error!("failed to get file from repo: {err}");
                Err(ReleasaurusError::forge(format!(
                    "failed to get file from repo: {err}"
                )))
            }
        }
    }

    async fn get_release_by_tag(
        &self,
        tag: &str,
    ) -> Result<ReleaseByTagResponse> {
        let tag_endpoint = GitlabTagBuilder::builder()
            .project(&self.project_id)
            .tag_name(tag)
            .build()?;

        let result: std::result::Result<
            GitlabTag,
            gitlab::api::ApiError<gitlab::RestError>,
        > = tag_endpoint.query_async(&self.gl).await;

        let tag: GitlabTag = match result {
            Ok(tag) => tag,
            Err(gitlab::api::ApiError::GitlabWithStatus { status, msg }) => {
                if status == StatusCode::NOT_FOUND {
                    return Err(ReleasaurusError::forge(format!(
                        "tag not found: {tag}"
                    )));
                }

                return Err(ReleasaurusError::forge(msg));
            }
            Err(err) => return Err(ReleasaurusError::forge(err.to_string())),
        };

        let endpoint = ProjectReleaseByTag::builder()
            .project(&self.project_id)
            .tag(tag.name.clone().into())
            .build()
            .map_err(|e| {
                ReleasaurusError::Other(color_eyre::Report::msg(format!(
                    "Builder error: {}",
                    e
                )))
            })?;

        let result: std::result::Result<
            GitlabRelease,
            gitlab::api::ApiError<gitlab::RestError>,
        > = endpoint.query_async(&self.gl).await;

        match result {
            Ok(release) => Ok(ReleaseByTagResponse {
                tag: tag.name,
                sha: tag.commit.id,
                notes: release.description,
            }),
            Err(gitlab::api::ApiError::GitlabWithStatus { status, msg }) => {
                if status == StatusCode::NOT_FOUND {
                    return Err(ReleasaurusError::forge(format!(
                        "no release found for tag: {}",
                        tag.name
                    )));
                }

                Err(ReleasaurusError::forge(msg))
            }
            Err(err) => Err(ReleasaurusError::forge(err.to_string())),
        }
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
                        timestamp: DateTime::parse_from_rfc3339(
                            &t.commit.created_at,
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
        branch: Option<String>,
        sha: Option<String>,
    ) -> Result<Vec<ForgeCommit>> {
        let branch = branch.clone().unwrap_or(self.default_branch());

        let search_depth = self.commit_search_depth.lock().await;

        let mut builder = Commits::builder();

        builder
            .project(&self.project_id)
            .order(CommitsOrder::Default);

        if let Some(sha) = sha.clone() {
            let range = format!("{sha}..{branch}");
            builder.ref_name(range);
        }

        let endpoint = builder.build()?;
        let page_limit =
            cmp::min(DEFAULT_PAGE_SIZE.into(), *search_depth) as usize;
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

        let mut forge_commits = vec![];

        for commit in result.iter() {
            debug!("backfilling file list for commit: {}", commit.id);

            let vars = CommitDiffQueryVars {
                project_id: self.project_id.clone(),
                commit_sha: commit.id.clone(),
            };

            let query = CommitDiffQuery::build_query(vars);

            let resp = self.gl.graphql::<CommitDiffQuery>(&query).await?;

            let diffs = resp.project.repository.commit.diffs;

            let mut files = vec![];

            for item in diffs.iter() {
                if let Some(file_path) = item.new_path.clone() {
                    files.push(file_path.clone());
                } else if let Some(file_path) = item.old_path.clone() {
                    files.push(file_path);
                }
            }

            let timestamp =
                DateTime::parse_from_rfc3339(&commit.created_at)?.timestamp();

            forge_commits.push(ForgeCommit {
                author_email: commit.author_email.clone(),
                author_name: commit.author_name.clone(),
                id: commit.id.clone(),
                short_id: commit
                    .id
                    .clone()
                    .split("")
                    .take(8)
                    .collect::<Vec<&str>>()
                    .join(""),
                link: commit.web_url.clone(),
                merge_commit: commit.parent_ids.len() > 1,
                message: commit.message.clone().trim().into(),
                timestamp,
                files,
            })
        }

        Ok(forge_commits)
    }

    async fn create_release_branch(
        &self,
        req: CreateReleaseBranchRequest,
    ) -> Result<Commit> {
        let mut actions: Vec<CommitAction> = vec![];

        for change in req.file_changes {
            let mut content = change.content;

            let mut update_type = CommitActionType::Update;

            let existing_content = self
                .get_file_content(GetFileContentRequest {
                    branch: Some(req.base_branch.clone()),
                    path: change.path.to_string(),
                })
                .await?;

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
            .start_branch(req.base_branch)
            .branch(req.release_branch)
            .actions(actions)
            .commit_message(req.message)
            .force(true)
            .build()?;

        let commit: CreatedCommit = endpoint.query_async(&self.gl).await?;

        Ok(Commit { sha: commit.id })
    }

    async fn create_commit(&self, req: CreateCommitRequest) -> Result<Commit> {
        let mut actions: Vec<CommitAction> = vec![];

        for change in req.file_changes {
            let mut content = change.content;

            let mut update_type = CommitActionType::Update;

            let existing_content = self
                .get_file_content(GetFileContentRequest {
                    branch: Some(req.target_branch.clone()),
                    path: change.path.to_string(),
                })
                .await?;

            if existing_content.is_none() {
                update_type = CommitActionType::Create;
            }

            if matches!(change.update_type, FileUpdateType::Prepend)
                && let Some(existing_content) = existing_content.clone()
            {
                content = format!("{content}{existing_content}");
            }

            if content == existing_content.unwrap_or_default() {
                warn!(
                    "skipping file update content matches existing state: {}",
                    change.path
                );
                continue;
            }

            let action = CommitAction::builder()
                .action(update_type)
                .content(content.as_bytes().to_owned())
                .file_path(change.path.clone())
                .build()?;

            actions.push(action)
        }

        if actions.is_empty() {
            warn!(
                "commit would result in no changes: target_branch: {}, message: {}",
                req.target_branch, req.message,
            );
            return Ok(Commit { sha: "None".into() });
        }

        let endpoint = CreateCommit::builder()
            .project(&self.project_id)
            .branch(&req.target_branch)
            .actions(actions)
            .commit_message(req.message)
            .force(false)
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
                    body: merge_request.description.clone(),
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
                    Err(ReleasaurusError::forge(msg))
                }
            }
            Err(err) => Err(ReleasaurusError::forge(format!(
                "encountered error querying gitlab for merge request: {err}"
            ))),
        }
    }

    async fn get_merged_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        info!(
            "looking for closed release prs with pending label for branch: {}",
            req.head_branch
        );

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
                "No merged release PRs with the label {PENDING_LABEL} found for branch {}",
                req.head_branch
            );
            return Ok(None);
        }

        if merge_requests.len() > 1 {
            return Err(ReleasaurusError::forge(format!(
                "Found more than one closed release PR with pending label for branch {}. \
                This means either release PRs were closed manually or releasaurus failed to remove tags. \
                You must remove the {PENDING_LABEL} label from all closed release PRs except for the most recent.",
                req.head_branch
            )));
        }

        let merge_request = &merge_requests[0];
        info!("found release pr: {}", merge_request.iid);

        // Check if the MR is actually merged (has merged_at timestamp)
        if merge_request.merged_at.is_none() {
            return Err(ReleasaurusError::forge(format!(
                "found release PR {} but it hasn't been merged yet",
                merge_request.iid
            )));
        }

        let sha = merge_request
            .merge_commit_sha
            .clone()
            .unwrap_or(merge_request.sha.clone());

        Ok(Some(PullRequest {
            number: merge_request.iid,
            sha,
            body: merge_request.description.clone(),
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
            body: merge_request.description.clone(),
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
