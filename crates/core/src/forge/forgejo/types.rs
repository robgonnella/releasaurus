use serde::{Deserialize, Serialize};

use crate::forge::request::Commit;

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ForgejoFileChangeOperation {
    Create,
    Update,
}

#[derive(Debug, Serialize)]
pub struct ForgejoFileChange {
    pub path: String,
    pub content: String,
    pub operation: ForgejoFileChangeOperation,
    pub sha: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ForgejoModifyFiles {
    pub branch: String,
    pub new_branch: Option<String>,
    pub message: String,
    pub files: Vec<ForgejoFileChange>,
    // TODO: add this once forgejo supports force pushing on /contents route
    // pub force_push: bool,
}

#[derive(Debug, Deserialize)]
pub struct ForgejoCreatedCommit {
    pub commit: Commit,
}
