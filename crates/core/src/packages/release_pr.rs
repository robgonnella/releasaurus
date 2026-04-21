use crate::forge::request::{FileChange, PullRequest, Tag};

/// Represents a fully analyzed and updated package ready for PR creation.
/// Includes next tag and list of file changes to include in PR
#[derive(Debug)]
pub struct ReleasePRPackage {
    pub name: String,
    pub tag: Tag,
    pub notes: String,
    pub tag_compare_link: String,
    pub sha_compare_link: String,
    pub file_changes: Vec<FileChange>,
    pub release_branch: String,
}

/// Groups the packages sharing a release branch with the existing open PR
/// for that branch, if one exists.
pub struct PRBundle {
    pub existing_pr: Option<PullRequest>,
    pub packages: Vec<ReleasePRPackage>,
}
