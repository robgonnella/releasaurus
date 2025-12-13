//! Common test helper functions shared across test modules.
//!
//! This module provides reusable utilities for creating test fixtures and mock objects,
//! reducing code duplication across different test suites.
use regex::Regex;
use secrecy::SecretString;
use semver::Version as SemVer;

use crate::{
    analyzer::{
        config::AnalyzerConfig,
        release::{Release, Tag},
    },
    config::{
        Config, changelog::ChangelogConfig, package::PackageConfig,
        release_type::ReleaseType,
    },
    forge::{
        config::RemoteConfig,
        request::{ForgeCommit, PullRequest},
    },
};

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
        release_link_base_url: "https://github.com/test/repo/releases/tag"
            .to_string(),
        dry_run: false,
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
///     PackageConfig {
///         name: "my-package".into(),
///         path: ".".into(),
///         workspace_root: ".".into(),
///         release_type: Some(ReleaseType::Node),
///         tag_prefix: Some("v".to_string()),
///         prerelease: None,
///         additional_paths: None,
///     }
/// ]);
/// ```
pub fn create_test_config(packages: Vec<PackageConfig>) -> Config {
    Config {
        first_release_search_depth: 100,
        separate_pull_requests: false,
        prerelease: None,
        prerelease_version: true,
        breaking_always_increment_major: true,
        features_always_increment_minor: true,
        custom_major_increment_regex: None,
        custom_minor_increment_regex: None,
        changelog: ChangelogConfig {
            body: "## Changes\n{{ commits }}".to_string(),
            skip_ci: false,
            skip_chore: false,
            skip_miscellaneous: false,
            skip_merge_commits: true,
            skip_release_commits: true,
            include_author: false,
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
pub fn create_test_config_simple(
    packages: Vec<(&str, &str, ReleaseType)>,
) -> Config {
    Config {
        first_release_search_depth: 100,
        separate_pull_requests: false,
        prerelease: None,
        prerelease_version: true,
        breaking_always_increment_major: true,
        features_always_increment_minor: true,
        custom_major_increment_regex: None,
        custom_minor_increment_regex: None,
        changelog: ChangelogConfig {
            body: "## Changes\n{{ commits }}".to_string(),
            skip_ci: false,
            skip_chore: false,
            skip_miscellaneous: false,
            skip_merge_commits: true,
            skip_release_commits: true,
            include_author: false,
        },
        packages: packages
            .into_iter()
            .map(|(name, path, release_type)| PackageConfig {
                name: name.into(),
                path: path.to_string(),
                workspace_root: ".".into(),
                release_type: Some(release_type),
                tag_prefix: None,
                prerelease: None,
                prerelease_version: None,
                breaking_always_increment_major: None,
                features_always_increment_minor: None,
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
                additional_paths: None,
                additional_manifest_files: None,
            })
            .collect(),
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
        timestamp: None,
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
        short_id: id.split("").take(8).collect::<Vec<&str>>().join(""),
        link: format!("https://github.com/test/repo/commit/{}", id),
        author_name: "Test Author".to_string(),
        author_email: "test@example.com".to_string(),
        merge_commit: false,
        message: message.to_string(),
        timestamp,
        files: vec![],
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
        body: "".into(),
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
                timestamp: None,
            })
        } else {
            None
        },
        link: String::new(),
        sha: "test-sha".to_string(),
        commits: vec![],
        include_author: false,
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
pub fn create_test_analyzer_config(
    tag_prefix: Option<String>,
) -> AnalyzerConfig {
    AnalyzerConfig {
        tag_prefix,
        body: "Release version {{ version }}".to_string(),
        skip_ci: false,
        skip_chore: false,
        skip_miscellaneous: false,
        skip_merge_commits: true,
        skip_release_commits: true,
        include_author: false,
        release_link_base_url: "https://github.com/test/repo/releases/tag"
            .to_string(),
        prerelease: None,
        prerelease_version: true,
        breaking_always_increment_major: true,
        features_always_increment_minor: true,
        custom_major_increment_regex: None,
        custom_minor_increment_regex: None,

        release_commit_matcher: Some(
            Regex::new(r#"chore\(main\): release test-package"#).unwrap(),
        ),
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
        let packages = vec![PackageConfig {
            name: "".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Node),
            tag_prefix: Some("v".to_string()),
            prerelease: None,
            prerelease_version: None,
            breaking_always_increment_major: None,
            features_always_increment_minor: None,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            additional_paths: None,
            additional_manifest_files: None,
        }];
        let config = create_test_config(packages);
        assert_eq!(config.packages.len(), 1);
        assert_eq!(config.packages[0].path, ".");
    }

    #[test]
    fn test_create_test_config_simple() {
        let config = create_test_config_simple(vec![
            ("", "packages/one", ReleaseType::Node),
            ("", "packages/two", ReleaseType::Rust),
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
        let config = create_test_analyzer_config(None);
        assert!(config.tag_prefix.is_none());
        assert!(!config.body.is_empty());
    }

    #[test]
    fn test_create_test_analyzer_config_with_prefix() {
        let config = create_test_analyzer_config(Some("v".to_string()));
        assert_eq!(config.tag_prefix, Some("v".to_string()));
    }
}
