use crate::{
    analyzer::commit::Commit,
    forge::request::{FileChange, Tag},
};

/// Represents a fully analyzed and updated package ready for PR creation.
/// Includes next tag and list of file changes to include in PR
pub struct ReleasePRPackage {
    pub name: String,
    pub tag: Tag,
    pub notes: String,
    /// Commits in this release, for the short notes the PR body falls
    /// back to when the full notes exceed the forge's PR body limit.
    pub commits: Vec<Commit>,
    pub tag_compare_link: String,
    pub sha_compare_link: String,
}

/// Shows the number of commits rather than the commits themselves, which
/// carry author emails and full messages and would flood debug logs.
impl std::fmt::Debug for ReleasePRPackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReleasePRPackage")
            .field("name", &self.name)
            .field("tag", &self.tag)
            .field("notes", &self.notes)
            .field("commits", &self.commits.len())
            .field("tag_compare_link", &self.tag_compare_link)
            .field("sha_compare_link", &self.sha_compare_link)
            .finish()
    }
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
