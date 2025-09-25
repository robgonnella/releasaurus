use serde::{Deserialize, Serialize};

#[allow(unused)]
#[derive(Debug, Clone)]
/// Release pull request information.
pub struct PullRequest {
    pub number: u64,
    pub sha: String,
}

#[derive(Debug, Clone)]
/// Request to get pull request by branch names.
pub struct GetPrRequest {
    pub head_branch: String,
    pub base_branch: String,
}

#[derive(Debug, Clone)]
/// Request to create a new pull request.
pub struct CreatePrRequest {
    pub head_branch: String,
    pub base_branch: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone)]
/// Request to update existing pull request.
pub struct UpdatePrRequest {
    pub pr_number: u64,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone)]
/// Request to update pull request labels.
pub struct PrLabelsRequest {
    pub pr_number: u64,
    pub labels: Vec<String>,
}

#[derive(Debug)]
/// Represents a normalized commit returned from any forge
pub struct ForgeCommit {
    pub id: String,
    pub link: String,
    pub author_name: String,
    pub author_email: String,
    pub merge_commit: bool,
    pub message: String,
    pub timestamp: i64,
}

#[allow(unused)]
#[derive(Debug, Clone, Serialize)]
pub enum FileUpdateType {
    Replace,
    Prepend,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileChange {
    /// Relative path to the file starting from repo root
    pub path: String,
    /// Contents of file changes. This tool only supports updating existing
    /// files, or creation of new files, but not deletion of files as there
    /// should never be a need to delete a file when generating a release-pr
    pub content: String,
    // Whether or not to replace the entire contents of file
    pub update_type: FileUpdateType,
}

#[derive(Debug, Clone)]
pub struct CreateBranchRequest {
    pub branch: String,
    pub message: String,
    pub file_changes: Vec<FileChange>,
}

#[derive(Debug, Deserialize)]
pub struct Commit {
    pub sha: String,
}
