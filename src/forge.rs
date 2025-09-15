//! Unified interface for Git forge platforms (GitHub, GitLab, Gitea).
//!
//! Provides token-based authentication, release management, pull request
//! operations, and repository information through common traits.

/// Configuration and authentication for forge platforms.
pub mod config;

/// Gitea and Forgejo API client implementation.
pub mod gitea;

/// GitHub API client implementation for GitHub.com and Enterprise.
pub mod github;

/// GitLab API client implementation for GitLab.com and self-hosted instances.
pub mod gitlab;

/// Common traits for forge platform abstraction.
pub mod traits;

/// Shared data types for releases, pull requests, and repository information.
pub mod types;
