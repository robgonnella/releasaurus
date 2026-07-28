use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct GithubTreeEntry {
    pub path: String,
    pub mode: String,
    pub content: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct GithubTree {
    pub base_tree: String,
    pub tree: Vec<GithubTreeEntry>,
}

#[derive(Debug, Deserialize)]
pub struct Tree {
    pub sha: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubCommitPRBase {
    #[serde(rename = "ref")]
    pub reference: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubCommitPR {
    pub number: u64,
    pub state: String,
    pub merged_at: Option<String>,
    pub html_url: String,
    pub base: GithubCommitPRBase,
}

pub const TREE_BLOB_MODE: &str = "100644";
pub const TREE_BLOB_TYPE: &str = "blob";
