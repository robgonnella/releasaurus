use serde::{Deserialize, Serialize};

use crate::forge::request::Commit;

#[derive(Debug, Default, Serialize)]
pub struct CreateLabel {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: u64,
    pub name: String,
    pub color: String,
    pub description: String,
    pub exclusive: bool,
    pub is_archived: bool,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestBranch {
    pub label: String,
    pub sha: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaPullRequest {
    pub number: u64,
    pub head: PullRequestBranch,
    pub merge_commit_sha: Option<String>,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaIssuePr {
    pub merged: bool,
}

#[derive(Debug, Deserialize)]
pub struct GiteaIssue {
    pub number: u64,
    pub pull_request: GiteaIssuePr,
}

#[derive(Debug, Serialize)]
pub struct CreatePull {
    pub title: String,
    pub body: String,
    pub head: String,
    pub base: String,
}

#[derive(Debug, Serialize)]
pub struct UpdatePullBody {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct UpdatePullLabels {
    pub labels: Vec<u64>,
}

#[derive(Debug, Serialize)]
pub struct CreateRelease {
    pub tag_name: String,
    pub target_commitish: String,
    pub name: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
}

#[derive(Debug, Deserialize)]
pub struct CommitAuthor {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct GiteaCommitParent {
    pub sha: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaCommit {
    pub author: CommitAuthor,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaCommitFile {
    pub filename: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaCommitQueryObject {
    pub sha: String,
    pub created: String,
    pub commit: GiteaCommit,
    pub files: Vec<GiteaCommitFile>,
    pub parents: Vec<GiteaCommitParent>,
    pub html_url: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaTagCommit {
    pub created: String,
    pub sha: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaTag {
    pub name: String,
    pub commit: GiteaTagCommit,
}

#[derive(Debug, Deserialize)]
pub struct GiteaRelease {
    pub body: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GiteaFileChangeOperation {
    Create,
    Update,
}

#[derive(Debug, Serialize)]
pub struct GiteaFileChange {
    pub path: String,
    pub content: String,
    pub operation: GiteaFileChangeOperation,
    pub sha: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GiteaModifyFiles {
    pub old_ref_name: String,
    pub new_branch: Option<String>,
    pub message: String,
    pub files: Vec<GiteaFileChange>,
    pub force: bool,
}

#[derive(Debug, Deserialize)]
pub struct GiteaCreatedCommit {
    pub commit: Commit,
}
