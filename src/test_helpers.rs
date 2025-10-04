//! Common test helper functions shared across test modules.
//!
//! This module provides reusable utilities for creating test fixtures and mock objects,
//! reducing code duplication across different test suites.

use crate::{
    analyzer::{
        config::AnalyzerConfig,
        release::{Release, Tag},
    },
    config::{ChangelogConfig, Config, PackageConfig, ReleaseType},
    forge::{
        config::RemoteConfig,
        request::{ForgeCommit, PullRequest},
    },
};
use secrecy::SecretString;
use semver::Version as SemVer;

/// Creates a test RemoteConfig with sensible defaults.
///
/// # Example
/// ```ignore
/// let config = create_test_remote_config();
/// ```
pub fn create_test_remote_config() -> RemoteConfig {
    RemoteConfig {
        host: "github.com".to_string(),
        port: None,
        scheme: "https".to_string(),
        owner: "test".to_string(),
        repo: "repo".to_string(),
        path: "test/repo".to_string(),
        token: SecretString::from("test-token".to_string()),
        commit_link_base_url: "https://github.com/test/repo/commit".to_string(),
        release_link_base_url: "https://github.com/test/repo/releases/tag"
            .to_string(),
    }
}

/// Creates a test Config with the provided packages.
///
/// # Arguments
/// * `packages` - Vector of package configurations
///
/// # Example
/// ```ignore
/// let config = create_test_config(vec![
///     create_test_package_config(".", Some(ReleaseType::Node), Some("v".to_string()))
/// ]);
/// ```
pub fn create_test_config(packages: Vec<PackageConfig>) -> Config {
    Config {
        first_release_search_depth: 100,
        changelog: ChangelogConfig {
            body: "## Changes\n{{ commits }}".to_string(),
        },
        packages,
    }
}

/// Creates a test Config from a list of (path, ReleaseType) tuples.
///
/// This is a convenience function for tests that don't need custom tag
/// prefixes.
///
/// # Arguments
/// * `packages` - Vector of (path, ReleaseType) tuples
///
/// # Example
/// ```ignore
/// let config = create_test_config_simple(vec![
///     ("packages/one", ReleaseType::Node),
///     ("packages/two", ReleaseType::Rust),
/// ]);
/// ```
pub fn create_test_config_simple(packages: Vec<(&str, ReleaseType)>) -> Config {
    Config {
        first_release_search_depth: 100,
        changelog: ChangelogConfig {
            body: "## Changes\n{{ commits }}".to_string(),
        },
        packages: packages
            .into_iter()
            .map(|(path, release_type)| PackageConfig {
                path: path.to_string(),
                release_type: Some(release_type),
                tag_prefix: None,
            })
            .collect(),
    }
}

/// Creates a test PackageConfig.
///
/// # Arguments
/// * `path` - Package path relative to repository root
/// * `release_type` - Optional release type
/// * `tag_prefix` - Optional custom tag prefix
///
/// # Example
/// ```ignore
/// let package = create_test_package_config(".", Some(ReleaseType::Node), Some("v".to_string()));
/// ```
pub fn create_test_package_config(
    path: &str,
    release_type: Option<ReleaseType>,
    tag_prefix: Option<String>,
) -> PackageConfig {
    PackageConfig {
        path: path.to_string(),
        release_type,
        tag_prefix,
    }
}

/// Creates a test Tag with the given parameters.
///
/// # Arguments
/// * `name` - Tag name (e.g., "v1.0.0")
/// * `semver` - Semantic version string (e.g., "1.0.0")
/// * `sha` - Git commit SHA
///
/// # Example
/// ```ignore
/// let tag = create_test_tag("v1.0.0", "1.0.0", "abc123");
/// ```
pub fn create_test_tag(name: &str, semver: &str, sha: &str) -> Tag {
    Tag {
        sha: sha.to_string(),
        name: name.to_string(),
        semver: SemVer::parse(semver).unwrap(),
    }
}

/// Creates a test ForgeCommit with the given parameters.
///
/// # Arguments
/// * `id` - Commit ID/SHA
/// * `message` - Commit message
/// * `timestamp` - Unix timestamp
///
/// # Example
/// ```ignore
/// let commit = create_test_forge_commit("abc123", "feat: add feature", 1000);
/// ```
pub fn create_test_forge_commit(
    id: &str,
    message: &str,
    timestamp: i64,
) -> ForgeCommit {
    ForgeCommit {
        id: id.to_string(),
        link: format!("https://github.com/test/repo/commit/{}", id),
        author_name: "Test Author".to_string(),
        author_email: "test@example.com".to_string(),
        merge_commit: false,
        message: message.to_string(),
        timestamp,
    }
}

/// Creates a test PullRequest with the given parameters.
///
/// # Arguments
/// * `number` - PR number
/// * `sha` - Merge commit SHA
///
/// # Example
/// ```ignore
/// let pr = create_test_pull_request(42, "merge-sha");
/// ```
pub fn create_test_pull_request(number: u64, sha: &str) -> PullRequest {
    PullRequest {
        number,
        sha: sha.to_string(),
    }
}

/// Creates a test Release with the given parameters.
///
/// # Arguments
/// * `version` - Semantic version string (e.g., "1.0.0")
/// * `has_tag` - Whether the release should include a tag
///
/// # Example
/// ```ignore
/// let release = create_test_release("1.0.0", true);
/// ```
pub fn create_test_release(version: &str, has_tag: bool) -> Release {
    Release {
        tag: if has_tag {
            Some(Tag {
                sha: "test-sha".to_string(),
                name: format!("v{}", version),
                semver: SemVer::parse(version).unwrap(),
            })
        } else {
            None
        },
        link: String::new(),
        sha: "test-sha".to_string(),
        commits: vec![],
        notes: String::new(),
        timestamp: 0,
    }
}

/// Creates a test AnalyzerConfig with sensible defaults.
///
/// # Example
/// ```ignore
/// let config = create_test_analyzer_config();
/// ```
pub fn create_test_analyzer_config() -> AnalyzerConfig {
    AnalyzerConfig {
        tag_prefix: None,
        body: "Release version {{ version }}".to_string(),
        release_link_base_url: "https://github.com/test/repo/releases/tag"
            .to_string(),
    }
}

/// Creates a test AnalyzerConfig with a custom tag prefix.
///
/// # Arguments
/// * `tag_prefix` - Optional tag prefix (e.g., "v", "api-v")
///
/// # Example
/// ```ignore
/// let config = create_test_analyzer_config_with_prefix(Some("v".to_string()));
/// ```
pub fn create_test_analyzer_config_with_prefix(
    tag_prefix: Option<String>,
) -> AnalyzerConfig {
    AnalyzerConfig {
        tag_prefix,
        body: "Release version {{ version }}".to_string(),
        release_link_base_url: "https://github.com/test/repo/releases/tag"
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_remote_config() {
        let config = create_test_remote_config();
        assert_eq!(config.host, "github.com");
        assert_eq!(config.owner, "test");
        assert_eq!(config.repo, "repo");
    }

    #[test]
    fn test_create_test_config() {
        let packages = vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )];
        let config = create_test_config(packages);
        assert_eq!(config.packages.len(), 1);
        assert_eq!(config.packages[0].path, ".");
    }

    #[test]
    fn test_create_test_config_simple() {
        let config = create_test_config_simple(vec![
            ("packages/one", ReleaseType::Node),
            ("packages/two", ReleaseType::Rust),
        ]);
        assert_eq!(config.packages.len(), 2);
        assert_eq!(config.packages[0].path, "packages/one");
        assert_eq!(config.packages[1].path, "packages/two");
    }

    #[test]
    fn test_create_test_tag() {
        let tag = create_test_tag("v1.0.0", "1.0.0", "abc123");
        assert_eq!(tag.name, "v1.0.0");
        assert_eq!(tag.sha, "abc123");
        assert_eq!(tag.semver.to_string(), "1.0.0");
    }

    #[test]
    fn test_create_test_forge_commit() {
        let commit = create_test_forge_commit("abc123", "feat: test", 1000);
        assert_eq!(commit.id, "abc123");
        assert_eq!(commit.message, "feat: test");
        assert_eq!(commit.timestamp, 1000);
    }

    #[test]
    fn test_create_test_pull_request() {
        let pr = create_test_pull_request(42, "merge-sha");
        assert_eq!(pr.number, 42);
        assert_eq!(pr.sha, "merge-sha");
    }

    #[test]
    fn test_create_test_release_with_tag() {
        let release = create_test_release("1.0.0", true);
        assert!(release.tag.is_some());
        assert_eq!(release.tag.unwrap().name, "v1.0.0");
    }

    #[test]
    fn test_create_test_release_without_tag() {
        let release = create_test_release("1.0.0", false);
        assert!(release.tag.is_none());
    }

    #[test]
    fn test_create_test_analyzer_config() {
        let config = create_test_analyzer_config();
        assert!(config.tag_prefix.is_none());
        assert!(!config.body.is_empty());
    }

    #[test]
    fn test_create_test_analyzer_config_with_prefix() {
        let config =
            create_test_analyzer_config_with_prefix(Some("v".to_string()));
        assert_eq!(config.tag_prefix, Some("v".to_string()));
    }
}
