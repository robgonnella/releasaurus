//! Data types for releases, tags, and commits.
use semver::Version;
use serde::{Deserialize, Serialize, ser::SerializeStruct};
use std::fmt::Display;

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
    /// Timestamp of tag
    pub timestamp: Option<i64>,
}

impl Default for Tag {
    fn default() -> Self {
        Self {
            name: "".into(),
            semver: Version::new(0, 0, 0),
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

#[derive(Debug, Deserialize)]
struct ShadowRelease {
    pub version: String,
    pub tag_name: String,
    pub link: String,
    pub sha: String,
    pub commits: Vec<Commit>,
    pub include_author: bool,
    pub notes: String,
    pub timestamp: i64,
}

/// Complete release package containing version tag, changelog notes, and all
/// associated commits for publishing.
#[derive(Clone, Default, Deserialize)]
#[serde(from = "ShadowRelease")]
pub struct Release {
    /// Associated version tag.
    pub tag: Tag,
    /// Release URL link.
    pub link: String,
    /// Git commit SHA for the release.
    pub sha: String,
    /// Commits included in this release.
    pub commits: Vec<Commit>,
    /// Whether or not to include author name for each commit in changelog
    pub include_author: bool,
    /// Generated release notes.
    pub notes: String,
    /// Release timestamp.
    pub timestamp: i64,
}

impl From<ShadowRelease> for Release {
    fn from(value: ShadowRelease) -> Self {
        Self {
            commits: value.commits,
            include_author: value.include_author,
            link: value.link,
            notes: value.notes,
            sha: value.sha,
            timestamp: value.timestamp,
            tag: Tag {
                name: value.tag_name,
                semver: semver::Version::parse(&value.version)
                    .unwrap_or(semver::Version::new(0, 0, 0)),
                sha: "".into(),
                timestamp: None,
            },
        }
    }
}

impl std::fmt::Debug for Release {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Release")
            .field("tag", &self.tag)
            .field("link", &self.link)
            .field("sha", &self.sha)
            .field("include_author", &self.include_author)
            .field("timestamp", &self.timestamp)
            .finish()
    }
}

impl Serialize for Release {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Release", 8)?;
        s.serialize_field("link", &self.link)?;
        s.serialize_field("version", &self.tag.semver.to_string())?;
        s.serialize_field("tag_name", &self.tag.name)?;
        s.serialize_field("sha", &self.sha)?;
        s.serialize_field("include_author", &self.include_author)?;
        s.serialize_field("commits", &self.commits)?;
        s.serialize_field("notes", &self.notes)?;
        s.serialize_field("timestamp", &self.timestamp)?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::commit::Commit;
    use semver::Version;
    use serde_json;

    // Tag tests

    #[test]
    fn tag_default_creates_empty_tag() {
        let tag = Tag::default();
        assert_eq!(tag.name, "");
        assert_eq!(tag.sha, "");
        assert_eq!(tag.semver, Version::new(0, 0, 0));
        assert_eq!(tag.timestamp, None);
    }

    #[test]
    fn tag_display_shows_name() {
        let tag = Tag {
            name: "v1.2.3".to_string(),
            sha: "abc123".to_string(),
            semver: Version::new(1, 2, 3),
            timestamp: Some(1234567890),
        };
        assert_eq!(format!("{}", tag), "v1.2.3");
    }

    #[test]
    fn tag_display_handles_empty_name() {
        let tag = Tag::default();
        assert_eq!(format!("{}", tag), "");
    }

    #[test]
    fn tag_serialize_includes_all_fields() {
        let tag = Tag {
            name: "v1.2.3".to_string(),
            sha: "abc123def456".to_string(),
            semver: Version::new(1, 2, 3),
            timestamp: Some(1234567890),
        };

        let json = serde_json::to_value(&tag).unwrap();
        assert_eq!(json["name"], "v1.2.3");
        assert_eq!(json["sha"], "abc123def456");
        assert_eq!(json["semver"], "1.2.3");
    }

    #[test]
    fn tag_serialize_with_prerelease() {
        let tag = Tag {
            name: "v2.0.0-beta.1".to_string(),
            sha: "xyz789".to_string(),
            semver: Version::parse("2.0.0-beta.1").unwrap(),
            timestamp: None,
        };

        let json = serde_json::to_value(&tag).unwrap();
        assert_eq!(json["semver"], "2.0.0-beta.1");
    }

    #[test]
    fn tag_equality_works() {
        let tag1 = Tag {
            name: "v1.0.0".to_string(),
            sha: "abc".to_string(),
            semver: Version::new(1, 0, 0),
            timestamp: Some(123),
        };
        let tag2 = Tag {
            name: "v1.0.0".to_string(),
            sha: "abc".to_string(),
            semver: Version::new(1, 0, 0),
            timestamp: Some(123),
        };
        assert_eq!(tag1, tag2);
    }

    #[test]
    fn tag_inequality_by_semver() {
        let tag1 = Tag {
            name: "v1.0.0".to_string(),
            sha: "abc".to_string(),
            semver: Version::new(1, 0, 0),
            timestamp: None,
        };
        let tag2 = Tag {
            name: "v1.0.1".to_string(),
            sha: "abc".to_string(),
            semver: Version::new(1, 0, 1),
            timestamp: None,
        };
        assert_ne!(tag1, tag2);
    }

    // Release tests

    #[test]
    fn release_debug_excludes_commits_and_notes() {
        let release = Release {
            tag: Tag {
                name: "v1.0.0".to_string(),
                sha: "tag_sha".to_string(),
                semver: Version::new(1, 0, 0),
                timestamp: Some(1234567890),
            },
            link: "https://example.com/release".to_string(),
            sha: "release_sha".to_string(),
            commits: vec![Commit::default()],
            include_author: true,
            notes: "Some long release notes...".to_string(),
            timestamp: 9876543210,
        };

        let debug_str = format!("{:?}", release);

        // Should include these fields
        assert!(debug_str.contains("Release"));
        assert!(debug_str.contains("tag"));
        assert!(debug_str.contains("link"));
        assert!(debug_str.contains("sha"));
        assert!(debug_str.contains("include_author"));
        assert!(debug_str.contains("timestamp"));

        // Should NOT include commits or notes in debug output
        assert!(!debug_str.contains("commits"));
        assert!(!debug_str.contains("notes"));
    }

    #[test]
    fn release_serialize_includes_all_fields() {
        let tag = Tag {
            name: "v2.1.0".to_string(),
            sha: "tag_sha_123".to_string(),
            semver: Version::new(2, 1, 0),
            timestamp: Some(1111111111),
        };

        let commit = Commit {
            id: "commit_sha".to_string(),
            raw_message: "feat: new feature".to_string(),
            ..Default::default()
        };

        let release = Release {
            tag,
            link: "https://github.com/owner/repo/releases/tag/v2.1.0"
                .to_string(),
            sha: "release_sha_456".to_string(),
            commits: vec![commit],
            include_author: true,
            notes: "# Release Notes\n\n- Added feature".to_string(),
            timestamp: 1234567890,
        };

        let json = serde_json::to_value(&release).unwrap();

        assert_eq!(
            json["link"],
            "https://github.com/owner/repo/releases/tag/v2.1.0"
        );
        assert_eq!(json["version"], "2.1.0");
        assert_eq!(json["sha"], "release_sha_456");
        assert_eq!(json["include_author"], true);
        assert!(json["commits"].is_array());
        assert_eq!(json["commits"].as_array().unwrap().len(), 1);
        assert_eq!(json["notes"], "# Release Notes\n\n- Added feature");
        assert_eq!(json["timestamp"], 1234567890);
    }

    #[test]
    fn release_serialize_empty_commits() {
        let release = Release {
            tag: Tag::default(),
            link: "".to_string(),
            sha: "".to_string(),
            commits: vec![],
            include_author: false,
            notes: "".to_string(),
            timestamp: 0,
        };

        let json = serde_json::to_value(&release).unwrap();
        assert!(json["commits"].is_array());
        assert_eq!(json["commits"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn release_with_multiple_commits() {
        let commits = vec![
            Commit {
                id: "sha1".to_string(),
                raw_message: "feat: feature 1".to_string(),
                ..Default::default()
            },
            Commit {
                id: "sha2".to_string(),
                raw_message: "fix: bug fix".to_string(),
                ..Default::default()
            },
            Commit {
                id: "sha3".to_string(),
                raw_message: "docs: update docs".to_string(),
                ..Default::default()
            },
        ];

        let release = Release {
            tag: Tag::default(),
            link: "".to_string(),
            sha: "".to_string(),
            commits,
            include_author: false,
            notes: "".to_string(),
            timestamp: 0,
        };

        assert_eq!(release.commits.len(), 3);
        assert_eq!(release.commits[0].id, "sha1");
        assert_eq!(release.commits[1].id, "sha2");
        assert_eq!(release.commits[2].id, "sha3");
    }
}
