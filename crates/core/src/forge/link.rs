//! Forge-specific construction of release and compare links.
//!
//! Each release carries two compare links, and both start at the previous
//! release tag:
//!
//! - the tag compare link targets the next release tag, which only exists
//!   once the release PR is merged and tagged, and is what ends up in the
//!   changelog;
//! - the sha compare link targets the release commit instead, so it is
//!   valid while the release PR is still open and is shown in the PR body
//!   in place of the tag compare link.
use url::Url;

use crate::result::Result;

/// How a forge encodes refs into its release and compare web links.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LinkStyle {
    /// Refs are appended to the base URL path: `{base}{tag}` for releases,
    /// `{base}{prev_tag}...{next_tag}` for tag compares and
    /// `{base}{prev_tag}...{sha}` for sha compares (GitHub, GitLab, Gitea).
    #[default]
    Path,
    /// Refs are passed as Azure DevOps version descriptors in the query,
    /// where the prefix selects the ref kind: `GT` for a tag, `GC` for a
    /// commit (`GB`, a branch, is not used). The release base is the
    /// repository web URL and the compare base is its `branchCompare` page:
    ///
    /// - release: `{repo}?path=/&version=GT{tag}`
    /// - tag compare:
    ///   `{repo}/branchCompare?baseVersion=GT{prev_tag}&targetVersion=GT{next_tag}`
    /// - sha compare:
    ///   `{repo}/branchCompare?baseVersion=GT{prev_tag}&targetVersion=GC{sha}`
    AzureDevops,
}

impl LinkStyle {
    /// Link to the web page for release tag `tag`.
    pub fn release_link(&self, base: &Url, tag: &str) -> Result<String> {
        match self {
            Self::Path => Ok(base.join(tag)?.to_string()),
            Self::AzureDevops => {
                let link = format!("{base}?path=/&version=GT{tag}");
                Ok(Url::parse(&link)?.to_string())
            }
        }
    }

    /// Link to the diff from the previous release tag `prev_tag` to the next
    /// release tag `next_tag`.
    pub fn tag_compare_link(
        &self,
        base: &Url,
        prev_tag: &str,
        next_tag: &str,
    ) -> Result<String> {
        match self {
            Self::Path => Self::path_compare_link(base, prev_tag, next_tag),
            Self::AzureDevops => {
                Self::azure_compare_link(base, prev_tag, "GT", next_tag)
            }
        }
    }

    /// Link to the diff from the previous release tag `prev_tag` to the
    /// release commit `sha`.
    pub fn sha_compare_link(
        &self,
        base: &Url,
        prev_tag: &str,
        sha: &str,
    ) -> Result<String> {
        match self {
            Self::Path => Self::path_compare_link(base, prev_tag, sha),
            Self::AzureDevops => {
                Self::azure_compare_link(base, prev_tag, "GC", sha)
            }
        }
    }

    fn path_compare_link(
        base: &Url,
        prev_tag: &str,
        target: &str,
    ) -> Result<String> {
        Ok(base.join(&format!("{prev_tag}...{target}"))?.to_string())
    }

    /// `target_kind` is the Azure DevOps version descriptor prefix of
    /// `target`: `GT` for a tag, `GC` for a commit.
    fn azure_compare_link(
        base: &Url,
        prev_tag: &str,
        target_kind: &str,
        target: &str,
    ) -> Result<String> {
        let link = format!(
            "{base}?baseVersion=GT{prev_tag}&targetVersion={target_kind}{target}"
        );
        Ok(Url::parse(&link)?.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn azure_web_url() -> Url {
        Url::parse("https://dev.azure.com/org/project/_git/repo").unwrap()
    }

    fn azure_compare_url() -> Url {
        Url::parse("https://dev.azure.com/org/project/_git/repo/branchCompare")
            .unwrap()
    }

    #[test]
    fn path_style_joins_refs_onto_base_path() {
        let base = Url::parse("https://github.com/o/r/compare/").unwrap();
        let style = LinkStyle::Path;

        assert_eq!(
            style.tag_compare_link(&base, "v1.0.0", "v1.1.0").unwrap(),
            "https://github.com/o/r/compare/v1.0.0...v1.1.0"
        );
        assert_eq!(
            style.sha_compare_link(&base, "v1.0.0", "abc123").unwrap(),
            "https://github.com/o/r/compare/v1.0.0...abc123"
        );

        let base = Url::parse("https://github.com/o/r/releases/tag/").unwrap();
        assert_eq!(
            style.release_link(&base, "v1.1.0").unwrap(),
            "https://github.com/o/r/releases/tag/v1.1.0"
        );
    }

    #[test]
    fn azure_style_tag_compare_uses_tag_descriptors() {
        let link = LinkStyle::AzureDevops
            .tag_compare_link(&azure_compare_url(), "v1.0.0", "v1.1.0")
            .unwrap();

        assert_eq!(
            link,
            "https://dev.azure.com/org/project/_git/repo/branchCompare\
             ?baseVersion=GTv1.0.0&targetVersion=GTv1.1.0"
        );
    }

    #[test]
    fn azure_style_sha_compare_uses_commit_descriptor() {
        let link = LinkStyle::AzureDevops
            .sha_compare_link(&azure_compare_url(), "v1.0.0", "abc123")
            .unwrap();

        assert_eq!(
            link,
            "https://dev.azure.com/org/project/_git/repo/branchCompare\
             ?baseVersion=GTv1.0.0&targetVersion=GCabc123"
        );
    }

    #[test]
    fn azure_style_release_link_keeps_repo_and_query() {
        let link = LinkStyle::AzureDevops
            .release_link(&azure_web_url(), "pkg/v1.1.0")
            .unwrap();

        assert_eq!(
            link,
            "https://dev.azure.com/org/project/_git/repo\
             ?path=/&version=GTpkg/v1.1.0"
        );
    }
}
