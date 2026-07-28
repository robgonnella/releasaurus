use derive_builder::Builder;
use gitlab::api::{Endpoint, common::NameOrId};
use reqwest::Method;
use serde::Deserialize;
use std::borrow::Cow;

#[derive(Debug, Deserialize)]
pub struct FileInfo {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct MergeRequestInfo {
    pub iid: u64,
    pub merge_commit_sha: Option<String>,
    pub squash_commit_sha: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct GitlabCommitMergeRequest {
    pub iid: u64,
    pub web_url: String,
    pub target_branch: String,
    pub state: String,
}

#[derive(Debug, Builder)]
#[builder(setter(strip_option))]
pub struct GitlabCommitMergeRequests<'a> {
    #[builder(setter(into))]
    project: NameOrId<'a>,

    #[builder(setter(into))]
    sha: &'a str,
}

impl<'a> GitlabCommitMergeRequests<'a> {
    pub fn builder() -> GitlabCommitMergeRequestsBuilder<'a> {
        GitlabCommitMergeRequestsBuilder::default()
    }
}

impl Endpoint for GitlabCommitMergeRequests<'_> {
    fn method(&self) -> Method {
        Method::GET
    }

    fn endpoint(&self) -> Cow<'static, str> {
        format!(
            "projects/{}/repository/commits/{}/merge_requests",
            self.project, self.sha
        )
        .into()
    }
}
