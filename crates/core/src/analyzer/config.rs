//! Configuration for changelog generation and commit analysis.

use derive_builder::Builder;
use url::Url;

use crate::config::{prerelease::PrereleaseConfig, resolved::CommitModifiers};

/// Configuration for commit analysis and changelog generation.
#[derive(Debug, Clone, Builder)]
#[builder(setter(into, strip_option), default)]
pub struct AnalyzerConfig {
    /// Tera template string for changelog body format.
    pub body: String,
    /// Skips including merge commits in changelog (default: true)
    pub skip_merge_commits: bool,
    /// Includes commit author in default body template (default: false)
    pub include_author: bool,
    /// Optional prefix for package tags.
    pub tag_prefix: Option<String>,
    /// Base URL for release links in changelog.
    pub release_link_base_url: Option<Url>,
    /// Base URL for comparing releases and showing diffs
    pub compare_link_base_url: Option<Url>,
    /// Prerelease settings (if enabled).
    pub prerelease: Option<PrereleaseConfig>,
    /// Always increments major version on breaking commits
    pub breaking_always_increment_major: bool,
    /// Always increments minor version on feature commits
    pub features_always_increment_minor: bool,
    /// Custom commit type regex matcher to increment major version
    pub custom_major_increment_regex: Option<String>,
    /// Custom commit type regex matcher to increment minor version
    pub custom_minor_increment_regex: Option<String>,
    /// Custom commit modifiers to skip commit shas or reword commit messages
    /// when generating changelog content
    pub commit_modifiers: CommitModifiers,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            body: "".into(),
            skip_merge_commits: true,
            include_author: false,
            tag_prefix: None,
            release_link_base_url: None,
            compare_link_base_url: None,
            prerelease: None,
            breaking_always_increment_major: true,
            features_always_increment_minor: true,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            commit_modifiers: CommitModifiers::default(),
        }
    }
}
