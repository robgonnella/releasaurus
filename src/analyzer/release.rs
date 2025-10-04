//! Data types for releases, tags, and commits.
use std::fmt::Display;

use color_eyre::eyre::ContextCompat;
use serde::{Serialize, ser::SerializeStruct};

use crate::analyzer::commit::Commit;

/// Git tag that represents a release version, linking a semantic version to
/// a specific commit SHA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// Git commit SHA of the tag.
    pub sha: String,
    /// Tag name.
    pub name: String,
    /// Semantic version parsed from tag name.
    pub semver: semver::Version,
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Tag", 3)?;
        s.serialize_field("sha", &self.sha)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("semver", &self.semver.to_string())?;
        s.end()
    }
}

/// Complete release package containing version tag, changelog notes, and all
/// associated commits for publishing.
#[derive(Clone, Default)]
pub struct Release {
    /// Associated version tag.
    pub tag: Option<Tag>,
    /// Release URL link.
    pub link: String,
    /// Git commit SHA for the release.
    pub sha: String,
    /// Commits included in this release.
    pub commits: Vec<Commit>,
    /// Generated release notes.
    pub notes: String,
    /// Release timestamp.
    pub timestamp: i64,
}

impl std::fmt::Debug for Release {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Release")
            .field("tag", &self.tag)
            .field("link", &self.link)
            .field("sha", &self.sha)
            .field("timestamp", &self.timestamp)
            .finish()
    }
}

impl Serialize for Release {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Release", 5)?;
        let tag = &self
            .tag
            .clone()
            .wrap_err("failed to find projected tag for release")
            .unwrap();
        s.serialize_field("link", &self.link)?;
        s.serialize_field("version", &tag.semver.to_string())?;
        s.serialize_field("sha", &self.sha)?;
        s.serialize_field("commits", &self.commits)?;
        s.serialize_field("timestamp", &self.timestamp)?;
        s.end()
    }
}
