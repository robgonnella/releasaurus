use crate::forge::request::{ForgeCommit, Tag};

/// Package ready for analysis, with commits filtered to those
/// relevant to this package since its last release tag.
#[derive(Debug)]
pub struct PreparedPackage {
    pub name: String,
    pub current_tag: Option<Tag>,
    pub commits: Vec<ForgeCommit>,
}
