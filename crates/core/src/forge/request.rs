use chrono::{DateTime, Utc};
use color_eyre::eyre::eyre;
use derive_builder::Builder;
use semver::BuildMetadata;
use serde::{Deserialize, Serialize, ser::SerializeStruct};
use std::{fmt::Display, hash::Hash};

use crate::{
    config::versioning::{VersionType, parse_strftime},
    result::{ReleasaurusError, Result},
};

/// Forge-specific encoding of PR metadata: content to embed inside the div
/// and an optional extra HTML attribute on the div tag.
#[derive(Debug)]
pub struct PrMetadataBlock {
    /// Content to embed inside the metadata div (e.g. an HTML comment).
    /// Empty when the forge strips HTML comments.
    pub inline_content: String,
    /// Extra HTML attribute to add to the div tag (e.g. `data-meta="..."`).
    /// Empty for forges that preserve inline HTML.
    pub div_attribute: String,
}

/// Release pull request information with PR number, sha, and body
#[allow(unused)]
#[derive(Debug)]
pub struct PullRequest {
    pub number: u64,
    pub sha: String,
    pub body: String,
}

/// Request to get content for a file path in the remote repo
#[derive(Debug, PartialEq)]
pub struct GetFileContentRequest {
    pub branch: Option<String>,
    pub path: String,
}

/// Request to find a pull request by comparing head and base branch names.
#[derive(Debug)]
pub struct GetPrRequest {
    pub head_branch: String,
    pub base_branch: String,
}

/// Request to create a new pull request with title and description.
#[derive(Debug)]
pub struct CreatePrRequest {
    pub head_branch: String,
    pub base_branch: String,
    pub title: String,
    pub body: String,
}

/// Request to update an existing pull request's title and body.
#[derive(Debug)]
pub struct UpdatePrRequest {
    pub pr_number: u64,
    pub title: String,
    pub body: String,
}

/// Response data for retrieving release by tag.
#[derive(Debug, Default, Serialize)]
pub struct TagResponse {
    pub tag: String,
    pub sha: String,
}

/// Response data for retrieving release by tag.
#[derive(Debug, Default, Serialize)]
pub struct ReleaseByTagResponse {
    pub tag: String,
    pub sha: String,
    pub notes: String,
}

/// Request to replace all labels on a pull request.
#[derive(Debug)]
pub struct PrLabelsRequest {
    pub pr_number: u64,
    pub labels: Vec<String>,
}

/// Request to publish a release on the forge.
#[derive(Debug)]
pub struct CreateReleaseRequest {
    pub tag: String,
    pub sha: String,
    pub notes: String,
    /// Marks the release as a prerelease on forges that support it.
    /// Derived from the version's semver pre-release field, not config.
    pub prerelease: bool,
}

/// How to apply file content changes during branch creation.
#[derive(Debug, Copy, Clone, Serialize, PartialEq)]
pub enum FileUpdateType {
    Replace,
    Prepend,
}

/// File modification for branch creation, supporting updates and new files.
#[derive(Debug, Clone, Serialize)]
pub struct FileChange {
    pub path: String,
    pub content: String,
    pub update_type: FileUpdateType,
}

#[derive(Debug, Copy, Clone)]
pub enum ResolvedFileChangeAction {
    Create,
    Update,
}

#[derive(Debug)]
pub struct ResolvedFileChange {
    pub repo_path: String,
    pub full_content: String,
    pub action: ResolvedFileChangeAction,
}

/// Request to create a branch using base_branch as starting point
#[derive(Debug)]
pub struct CreateReleaseBranchRequest {
    pub base_branch: String,
    pub release_branch: String,
    pub message: String,
    pub file_changes: Vec<FileChange>,
}

/// Request to create a branch using base_branch as starting point
#[derive(Debug)]
pub struct ResolvedCreateReleaseBranchRequest {
    pub base_branch: String,
    pub release_branch: String,
    pub message: String,
    pub file_changes: Vec<ResolvedFileChange>,
}

/// Request to create a new commit on a branch with file changes
#[derive(Debug)]
pub struct CreateCommitRequest {
    pub target_branch: String,
    pub message: String,
    pub file_changes: Vec<FileChange>,
}

/// Request to create a new commit on a branch with file changes
#[derive(Debug)]
pub struct ResolvedCreateCommitRequest {
    pub target_branch: String,
    pub message: String,
    pub file_changes: Vec<ResolvedFileChange>,
}

/// Minimal commit information returned from forge API responses.
#[derive(Debug, Deserialize)]
pub struct Commit {
    pub sha: String,
}

/// `Strftime` is truncated to the precision its format expresses, so tags
/// that render alike compare equal.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TagVersion {
    Semantic(semver::Version),
    Strftime(DateTime<Utc>, String),
}

impl TagVersion {
    pub fn new_semantic(major: u64, minor: u64, patch: u64) -> Self {
        TagVersion::Semantic(semver::Version::new(major, minor, patch))
    }

    pub fn parse(raw: &str, version_type: &VersionType) -> Result<Self> {
        match version_type {
            VersionType::Semantic
            | VersionType::SemanticWithBuild
            | VersionType::SemanticDate
            | VersionType::SemanticDateWithTime
            | VersionType::SemanticDateWithTimeMicro => {
                Ok(Self::Semantic(semver::Version::parse(raw)?))
            }
            VersionType::Strftime(fmt) => {
                Ok(Self::Strftime(parse_strftime(raw, fmt)?, fmt.clone()))
            }
        }
    }

    /// Now, truncated to the precision `fmt` expresses.
    pub fn strftime_now(fmt: &str) -> Result<Self> {
        let rendered = Utc::now().format(fmt).to_string();
        Ok(Self::Strftime(
            parse_strftime(&rendered, fmt)?,
            fmt.to_string(),
        ))
    }

    pub fn semver(&self) -> Result<&semver::Version> {
        match self {
            Self::Semantic(v) => Ok(v),
            Self::Strftime(..) => Err(ReleasaurusError::Other(eyre!(
                "tag version {self} is not a semantic version"
            ))),
        }
    }

    pub fn pre(&self) -> &str {
        match self {
            Self::Semantic(v) => v.pre.as_str(),
            Self::Strftime(..) => "",
        }
    }

    pub fn build(&self) -> &str {
        match self {
            Self::Semantic(v) => v.build.as_str(),
            Self::Strftime(..) => "",
        }
    }

    pub fn set_build(&mut self, meta: BuildMetadata) {
        if let Self::Semantic(v) = self {
            v.build = meta;
        }
    }
}

impl Display for TagVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Semantic(v) => write!(f, "{v}"),
            Self::Strftime(d, fmt) => write!(f, "{}", d.format(fmt)),
        }
    }
}

/// Git tag that represents a release version, linking a semantic version to
/// a specific commit SHA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// Git commit SHA of the tag.
    pub sha: String,
    /// Tag name.
    pub name: String,
    /// Version parsed from the tag name.
    pub version: TagVersion,
    /// Timestamp of tag
    pub timestamp: Option<i64>,
}

impl Default for Tag {
    fn default() -> Self {
        Self {
            name: "".into(),
            version: TagVersion::new_semantic(0, 0, 0),
            sha: "".into(),
            timestamp: None,
        }
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl Serialize for Tag {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Tag", 3)?;
        s.serialize_field("sha", &self.sha)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("version", &self.version.to_string())?;
        s.end()
    }
}

/// Represents the PR that introduced the commit to history
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ForgeCommitPR {
    pub id: String,
    pub link: String,
}

/// Normalized commit data returned from any forge platform with metadata
/// and links.
#[derive(Debug, Clone, Default, Eq, Builder)]
#[builder(setter(into, strip_option), default)]
pub struct ForgeCommit {
    pub id: String,
    pub short_id: String,
    pub link: String,
    pub author_name: String,
    pub author_email: String,
    pub merge_commit: bool,
    pub message: String,
    pub timestamp: i64,
    pub files: Vec<String>,
    pub pr: Option<ForgeCommitPR>,
}

impl PartialEq for ForgeCommit {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for ForgeCommit {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_strftime_tag_renders_back_to_the_same_name() {
        for (name, fmt) in [
            ("2026.10.05", "%Y.%m.%d"),
            ("26.10.5", "%y.%-m.%-d"),
            ("2026.10", "%Y.%m"),
            ("20261005.1307", "%Y%m%d.%H%M"),
            ("20261005130709123456789", "%Y%m%d%H%M%S%f"),
            ("2026.10.05.130709123456", "%Y.%m.%d.%H%M%S%6f"),
            ("2026.10.05.130709.123", "%Y.%m.%d.%H%M%S%.3f"),
        ] {
            let version_type = VersionType::Strftime(fmt.to_string());
            let version = TagVersion::parse(name, &version_type).unwrap();
            assert_eq!(version.to_string(), name, "{fmt:?}");
        }
    }

    #[test]
    fn strftime_tags_sort_chronologically() {
        let version_type = VersionType::Strftime("%y.%-m.%-d".to_string());
        let mut tags = ["26.10.5", "25.12.31", "26.2.1"]
            .map(|name| TagVersion::parse(name, &version_type).unwrap());
        tags.sort();
        assert_eq!(
            tags.map(|t| t.to_string()),
            ["25.12.31", "26.2.1", "26.10.5"]
        );
    }

    #[test]
    fn strftime_now_is_truncated_to_format_precision() {
        let now = TagVersion::strftime_now("%Y.%m.%d").unwrap();
        let TagVersion::Strftime(instant, _) = &now else {
            panic!("expected strftime version");
        };
        assert_eq!(
            *instant,
            instant.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc()
        );

        let version_type = VersionType::Strftime("%Y.%m.%d".to_string());
        let from_name =
            TagVersion::parse(&now.to_string(), &version_type).unwrap();
        assert_eq!(now, from_name);
    }

    #[test]
    fn strftime_tag_has_no_semver_prerelease_or_build() {
        let version_type = VersionType::Strftime("%Y.%m.%d".to_string());
        let mut version =
            TagVersion::parse("2026.10.05", &version_type).unwrap();

        assert!(matches!(version.semver(), Err(ReleasaurusError::Other(_))));
        assert_eq!(version.pre(), "");
        assert_eq!(version.build(), "");

        version.set_build(BuildMetadata::new("1").unwrap());
        assert_eq!(version.build(), "");
        assert_eq!(version.to_string(), "2026.10.05");
    }

    #[test]
    fn parse_semantic_types_use_semver() {
        for version_type in [
            VersionType::Semantic,
            VersionType::SemanticWithBuild,
            VersionType::SemanticDate,
            VersionType::SemanticDateWithTime,
            VersionType::SemanticDateWithTimeMicro,
        ] {
            let version =
                TagVersion::parse("2026.10.5-rc.1+13.07.09", &version_type)
                    .unwrap();
            assert_eq!(
                version,
                TagVersion::Semantic(
                    semver::Version::parse("2026.10.5-rc.1+13.07.09").unwrap()
                )
            );
            assert_eq!(version.pre(), "rc.1");
            assert_eq!(version.build(), "13.07.09");
        }

        // leading zero is not semver
        assert!(
            TagVersion::parse("2026.10.05", &VersionType::Semantic).is_err()
        );
    }
}
