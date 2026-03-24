use crate::{analyzer::release::Tag, forge::request::ForgeCommit};

/// Represents a prepared package with pre-filtered commits list
#[derive(Debug)]
pub struct PreparedPackage {
    pub name: String,
    pub current_tag: Option<Tag>,
    pub commits: Vec<ForgeCommit>,
}
