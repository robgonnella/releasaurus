use crate::forge::request::{FileChange, Tag};

/// Represents a fully analyzed and updated package ready for PR creation.
/// Includes next tag and list of file changes to include in PR
#[derive(Debug)]
pub struct ReleasePRPackage {
    pub name: String,
    pub tag: Tag,
    pub notes: String,
    pub tag_compare_link: String,
    pub sha_compare_link: String,
}

/// Everything one release branch needs: the packages it carries, the file
/// changes they imply, and the rendered commit message and PR title.
///
/// File changes belong to the branch rather than to any one package: a
/// workspace lock file is shared, so it can only be written once per
/// branch.
pub struct PRBundle {
    pub release_branch: String,
    pub commit_message: String,
    pub pr_title: String,
    pub packages: Vec<ReleasePRPackage>,
    pub file_changes: Vec<FileChange>,
}
