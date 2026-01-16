use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FileInfo {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct MergeRequestInfo {
    pub iid: u64,
    pub merge_commit_sha: Option<String>,
    pub sha: String,
    pub merged_at: Option<String>,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct LabelInfo {
    pub name: String,
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
