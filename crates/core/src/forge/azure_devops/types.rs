use serde::{Deserialize, Serialize};

/// Top-level response wrapper used by most Azure DevOps list endpoints.
#[derive(Debug, Deserialize)]
pub struct AzureList<T> {
    /// Deserialized items returned by the endpoint.
    #[serde(default = "Vec::new")]
    pub value: Vec<T>,
}

/// Metadata for an Azure DevOps Git repository.
#[derive(Debug, Deserialize)]
pub struct AzureRepo {
    /// Fully-qualified ref name of the repository's default branch (e.g. `refs/heads/main`).
    #[serde(rename = "defaultBranch")]
    pub default_branch: Option<String>,
}

/// A Git ref (branch or tag) as returned by the Azure DevOps Refs API.
#[derive(Debug, Deserialize)]
pub struct AzureRef {
    /// Fully-qualified ref name (e.g. `refs/heads/main`).
    pub name: String,
    /// SHA-1 commit ID that this ref currently points to.
    #[serde(rename = "objectId")]
    pub object_id: String,
}

/// Describes a ref update to include in a push or Refs API request.
#[derive(Debug, Serialize)]
pub struct RefUpdate {
    /// Fully-qualified ref name to update (e.g. `refs/heads/main`).
    pub name: String,
    /// Current commit ID the ref points to; used for optimistic concurrency.
    #[serde(rename = "oldObjectId")]
    pub old_object_id: String,
    /// Required for the Refs API (create/update/delete refs).
    /// Omit for the Push API — Azure DevOps computes it from the commits.
    #[serde(rename = "newObjectId", skip_serializing_if = "Option::is_none")]
    pub new_object_id: Option<String>,
}

/// Author or committer information attached to an Azure DevOps commit.
#[derive(Debug, Default, Deserialize)]
pub struct AzureUser {
    /// Display name of the user.
    #[serde(default)]
    pub name: String,
    /// Email address of the user.
    #[serde(default)]
    pub email: String,
    /// ISO-8601 timestamp for the author or committer.
    #[serde(default)]
    pub date: String,
}

/// A Git commit as returned by the Azure DevOps Git Commits API.
#[derive(Debug, Deserialize)]
pub struct AzureCommit {
    /// SHA-1 identifier of the commit.
    #[serde(rename = "commitId")]
    pub commit_id: String,
    /// Commit author metadata.
    #[serde(default)]
    pub author: AzureUser,
    /// First line of the commit message.
    #[serde(default)]
    pub comment: String,
    /// SHA-1 identifiers of parent commits.
    #[serde(default)]
    pub parents: Vec<String>,
    /// Web URL to view the commit in Azure DevOps.
    #[serde(rename = "remoteUrl", default)]
    pub remote_url: String,
}

/// Path metadata for a single file affected by a commit change.
#[derive(Debug, Default, Deserialize)]
pub struct AzureChangeItem {
    /// Repository-relative path of the file (e.g. `/src/main.rs`).
    #[serde(default)]
    pub path: String,
}

/// A single file change entry within an [`AzureCommitChanges`] response.
#[derive(Debug, Deserialize)]
pub struct AzureChange {
    /// File path metadata for this change.
    #[serde(default)]
    pub item: AzureChangeItem,
}

/// Collection of file changes associated with a single commit.
#[derive(Debug, Deserialize)]
pub struct AzureCommitChanges {
    /// Individual file changes introduced by the commit.
    #[serde(default)]
    pub changes: Vec<AzureChange>,
}

/// File path descriptor used in push request bodies.
#[derive(Debug, Serialize)]
pub struct ChangeItem {
    /// Repository-relative path of the file (e.g. `/src/main.rs`).
    pub path: String,
}

/// New file content to be written as part of a push change.
#[derive(Debug, Serialize)]
pub struct NewContent {
    /// Raw file content (base64-encoded or plain-text depending on `content_type`).
    pub content: String,
    /// Encoding of `content`; typically `"rawtext"` or `"base64Encoded"`.
    #[serde(rename = "contentType")]
    pub content_type: String,
}

/// A single file operation to include in a push commit.
#[derive(Debug, Serialize)]
pub struct Change {
    /// Type of change: `"add"`, `"edit"`, or `"delete"`.
    #[serde(rename = "changeType")]
    pub change_type: String,
    /// Target file path for this change.
    pub item: ChangeItem,
    /// New file content; omitted for `"delete"` changes.
    #[serde(rename = "newContent", skip_serializing_if = "Option::is_none")]
    pub new_content: Option<NewContent>,
}

/// A commit to be created as part of a push operation.
#[derive(Debug, Serialize)]
pub struct PushCommit {
    /// Commit message.
    pub comment: String,
    /// File changes included in this commit.
    pub changes: Vec<Change>,
}

/// Request body for the Azure DevOps Git Push API.
#[derive(Debug, Serialize)]
pub struct Push {
    /// Refs to advance as a result of the push.
    #[serde(rename = "refUpdates")]
    pub ref_updates: Vec<RefUpdate>,
    /// Commits to create in the push, in order.
    pub commits: Vec<PushCommit>,
}

/// Abbreviated commit descriptor returned inside a push response.
#[derive(Debug, Deserialize)]
pub struct PushResponseCommit {
    /// SHA-1 identifier of the newly created commit.
    #[serde(rename = "commitId")]
    pub commit_id: String,
}

/// Response body returned by the Azure DevOps Git Push API.
#[derive(Debug, Deserialize)]
pub struct PushResponse {
    /// Commits created by the push.
    #[serde(default)]
    pub commits: Vec<PushResponseCommit>,
}

/// Request body for creating a new pull request.
#[derive(Debug, Serialize)]
pub struct CreatePullRequest {
    /// Fully-qualified source ref name (e.g. `refs/heads/feature-branch`).
    #[serde(rename = "sourceRefName")]
    pub source_ref_name: String,
    /// Fully-qualified target ref name (e.g. `refs/heads/main`).
    #[serde(rename = "targetRefName")]
    pub target_ref_name: String,
    /// Pull request title.
    pub title: String,
    /// Pull request description (supports Markdown).
    pub description: String,
}

/// Request body for updating an existing pull request's title or description.
#[derive(Debug, Serialize)]
pub struct UpdatePullRequest {
    /// New pull request title.
    pub title: String,
    /// New pull request description (supports Markdown).
    pub description: String,
}

/// A pull request as returned by the Azure DevOps Pull Requests API.
#[derive(Debug, Deserialize)]
pub struct AzurePullRequest {
    /// Numeric identifier of the pull request within the repository.
    #[serde(rename = "pullRequestId")]
    pub pull_request_id: u64,
    /// Fully-qualified target ref name (e.g. `refs/heads/main`).
    #[serde(rename = "targetRefName")]
    pub target_ref_name: String,
    /// Tip commit of the source branch at the time of the last merge attempt.
    #[serde(rename = "lastMergeSourceCommit", default)]
    pub last_merge_source_commit: Option<AzureCommitRef>,
    /// Merge commit produced by the last merge attempt.
    #[serde(rename = "lastMergeCommit", default)]
    pub last_merge_commit: Option<AzureCommitRef>,
    /// Pull request description (supports Markdown).
    #[serde(default)]
    pub description: Option<String>,
}

/// A lightweight commit reference (ID only) embedded in pull request responses.
#[derive(Debug, Deserialize, Clone)]
pub struct AzureCommitRef {
    /// SHA-1 identifier of the referenced commit.
    #[serde(rename = "commitId")]
    pub commit_id: String,
}

/// A label attached to an Azure DevOps pull request.
#[derive(Debug, Deserialize, Clone)]
pub struct AzureLabel {
    /// Unique identifier of the label.
    pub id: String,
    /// Display name of the label.
    pub name: String,
    /// Whether the label is currently active on the pull request.
    #[serde(default)]
    pub active: bool,
}

/// Request body for adding a label to a pull request.
#[derive(Debug, Serialize)]
pub struct CreateLabel {
    /// Name of the label to create or attach.
    pub name: String,
}
