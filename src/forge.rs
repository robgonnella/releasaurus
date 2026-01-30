//! Unified interface for Git forge platforms (GitHub, GitLab, Gitea).
//!
//! Provides token-based authentication, release management, pull request
//! operations, and repository information through common traits.

/// Configuration and authentication for forge platforms.
pub mod config;
pub mod gitea;
pub mod github;
pub mod gitlab;
pub mod local;
pub mod manager;
pub mod request;
pub mod traits;

#[cfg(test)]
#[cfg(feature = "_integration_tests")]
mod tests;
