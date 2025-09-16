//! Data types for releases, tags, and commits.
use std::fmt::Display;

use color_eyre::eyre::ContextCompat;
use serde::{Serialize, ser::SerializeStruct};

use crate::analyzer::commit::Commit;

/// Git tag with associated commit and semantic version information.
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

/// Release information with commits, notes, and version tag.
#[derive(Debug, Clone, Default)]
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

impl Serialize for Release {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Release", 4)?;
        let tag = &self
            .tag
            .clone()
            .wrap_err("failed to find projected tag for release")
            .unwrap();
        s.serialize_field("link", &self.link)?;
        s.serialize_field("version", &tag.semver.to_string())?;
        s.serialize_field("commits", &self.commits)?;
        s.serialize_field("timestamp", &self.timestamp)?;
        s.end()
    }
}
