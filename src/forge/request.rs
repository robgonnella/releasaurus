use serde::{Deserialize, Serialize};

/// Release pull request information with PR number and merge commit SHA.
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct PullRequest {
    pub number: u64,
    pub sha: String,
}

/// Request to find a pull request by comparing head and base branch names.
#[derive(Debug, Clone)]
pub struct GetPrRequest {
    pub head_branch: String,
    pub base_branch: String,
}

/// Request to create a new pull request with title and description.
#[derive(Debug, Clone)]
pub struct CreatePrRequest {
    pub head_branch: String,
    pub base_branch: String,
    pub title: String,
    pub body: String,
}

/// Request to update an existing pull request's title and body.
#[derive(Debug, Clone)]
pub struct UpdatePrRequest {
    pub pr_number: u64,
    pub title: String,
    pub body: String,
}

/// Request to replace all labels on a pull request.
#[derive(Debug, Clone)]
pub struct PrLabelsRequest {
    pub pr_number: u64,
    pub labels: Vec<String>,
}

/// Normalized commit data returned from any forge platform with metadata
/// and links.
#[derive(Debug)]
pub struct ForgeCommit {
    pub id: String,
    pub link: String,
    pub author_name: String,
    pub author_email: String,
    pub merge_commit: bool,
    pub message: String,
    pub timestamp: i64,
}

/// How to apply file content changes during branch creation.
#[allow(unused)]
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum FileUpdateType {
    Replace,
    Prepend,
}

/// File modification for branch creation, supporting updates and new files.
#[derive(Debug, Clone, Serialize)]
pub struct FileChange {
    /// Relative path to the file starting from repo root.
    pub path: String,
    /// File content to write or prepend.
    pub content: String,
    /// Whether to replace entire file or prepend to existing content.
    pub update_type: FileUpdateType,
}

/// Request to create a new branch with file changes and commit message.
#[derive(Debug, Clone)]
pub struct CreateBranchRequest {
    pub branch: String,
    pub message: String,
    pub file_changes: Vec<FileChange>,
}

/// Minimal commit information returned from forge API responses.
#[derive(Debug, Deserialize)]
pub struct Commit {
    pub sha: String,
}
