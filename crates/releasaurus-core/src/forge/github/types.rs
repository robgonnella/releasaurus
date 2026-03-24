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

pub const TREE_BLOB_MODE: &str = "100644";
pub const TREE_BLOB_TYPE: &str = "blob";
