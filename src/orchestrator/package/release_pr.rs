use crate::{analyzer::release::Tag, forge::request::FileChange};

/// Represents a fully analyzed and updated package ready for PR creation.
/// Includes next tag and list of file changes to include in PR
#[derive(Debug)]
pub struct ReleasePRPackage {
    pub name: String,
    pub tag: Tag,
    pub notes: String,
    pub file_changes: Vec<FileChange>,
    pub release_branch: String,
}
