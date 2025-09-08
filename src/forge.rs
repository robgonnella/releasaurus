//! Git forge integration and platform abstraction for Releasaurus.
//!
//! This module provides a unified interface for interacting with different Git forge
//! platforms (GitHub, GitLab, Gitea) through a common trait system. It abstracts away
//! platform-specific API differences, enabling Releasaurus to work seamlessly across
//! multiple hosting providers with consistent behavior.
//!
//! # Supported Platforms
//!
//! - **GitHub**: Full support for GitHub.com and GitHub Enterprise
//! - **GitLab**: Support for GitLab.com and self-hosted GitLab instances
//! - **Gitea**: Support for Gitea and Forgejo self-hosted instances
//!
//! # Key Features
//!
//! - **Unified API**: Common interface across all supported platforms
//! - **Authentication**: Token-based authentication with automatic credential management
//! - **Release Management**: Create, update, and manage releases
//! - **Pull Request Operations**: Create and manage pull/merge requests
//! - **Repository Information**: Access repository metadata and configuration
//! - **Error Handling**: Platform-specific error translation to common error types
//!
//! # Architecture
//!
//! The forge system is built around a trait-based architecture that allows for
//! easy extension and platform-specific customization while maintaining a consistent
//! interface for the rest of the application.
//!
//! ## Core Components
//!
//! - **Traits**: Define the common interface that all forge implementations must provide
//! - **Types**: Common data structures used across all platforms
//! - **Config**: Configuration and authentication management
//! - **Implementations**: Platform-specific API clients and logic
//!
//! ## Trait System
//!
//! The main trait `ForgeClient` defines all operations that can be performed
//! against a forge platform:
//!
//! ```rust,ignore
//! pub trait ForgeClient {
//!     async fn create_release(&self, release: &Release) -> Result<String>;
//!     async fn create_pull_request(&self, pr: &PullRequest) -> Result<String>;
//!     async fn get_repository_info(&self) -> Result<RepositoryInfo>;
//!     // ... additional methods
//! }
//! ```
//!
//! # Authentication
//!
//! All platforms use token-based authentication:
//!
//! - **GitHub**: Personal Access Tokens (classic or fine-grained)
//! - **GitLab**: Personal Access Tokens or Project Access Tokens
//! - **Gitea**: Access Tokens generated in user settings
//!
//! Tokens are provided through CLI arguments, environment variables, or embedded
//! in repository URLs, with automatic detection and secure handling.
//!
//! # Usage Patterns
//!
//! ## Direct Platform Usage
//!
//! ```rust,ignore
//! use crate::forge::github::GitHubClient;
//!
//! let client = GitHubClient::new(config)?;
//! let release = client.create_release(&release_data).await?;
//! ```
//!
//! ## Generic Platform Usage
//!
//! ```rust,ignore
//! use crate::forge::traits::ForgeClient;
//!
//! fn handle_release<T: ForgeClient>(client: &T, release: &Release) -> Result<()> {
//!     let release_url = client.create_release(release).await?;
//!     println!("Created release: {}", release_url);
//!     Ok(())
//! }
//! ```
//!
//! # Error Handling
//!
//! The forge system translates platform-specific errors into common error types,
//! providing consistent error handling across different platforms. This includes:
//!
//! - Authentication errors
//! - Network connectivity issues
//! - API rate limiting
//! - Resource not found errors
//! - Permission denied errors
//!
//! # Module Organization
//!
//! Each platform implementation follows a consistent structure:
//!
//! - **Client Structure**: Main API client with authentication
//! - **Request/Response Types**: Platform-specific data structures
//! - **Error Mapping**: Translation of platform errors to common types
//! - **Trait Implementation**: Implementation of the `ForgeClient` trait
//!
//! # Platform-Specific Features
//!
//! While the common interface provides consistent functionality, each platform
//! may have unique features or limitations:
//!
//! ## GitHub
//! - Draft releases support
//! - Release asset uploads
//! - Advanced pull request features
//! - GitHub Apps authentication (future)
//!
//! ## GitLab
//! - Merge request approval rules
//! - Pipeline integration
//! - Group-level access tokens
//! - Self-hosted instance support
//!
//! ## Gitea
//! - Lightweight, fast operations
//! - Self-hosted focused
//! - Simplified release model
//! - Forgejo compatibility
//!
//! # Configuration
//!
//! Each forge platform is configured through a `RemoteConfig` structure that
//! contains all necessary connection and authentication information:
//!
//! ```rust,ignore
//! let config = RemoteConfig {
//!     host: "github.com".to_string(),
//!     owner: "user".to_string(),
//!     repo: "repository".to_string(),
//!     token: SecretString::from("ghp_..."),
//!     // ... additional fields
//! };
//! ```

/// Configuration structures and remote repository settings for forge platforms.
///
/// Contains the core configuration types used to establish connections and
/// authenticate with different Git forge platforms. Handles URL parsing,
/// credential management, and connection parameters.
pub mod config;

/// Gitea forge platform integration.
///
/// Provides API client and implementation for Gitea and Forgejo instances.
/// Supports self-hosted Gitea deployments with full release and pull request
/// management capabilities.
pub mod gitea;

/// GitHub forge platform integration.
///
/// Provides API client and implementation for GitHub.com and GitHub Enterprise.
/// Includes support for advanced GitHub features like draft releases, asset
/// uploads, and comprehensive pull request management.
pub mod github;

/// GitLab forge platform integration.
///
/// Provides API client and implementation for GitLab.com and self-hosted
/// GitLab instances. Supports GitLab-specific features like merge request
/// approval workflows and pipeline integration.
pub mod gitlab;

/// Common traits and interfaces for forge platform abstraction.
///
/// Defines the core `ForgeClient` trait and related interfaces that all
/// platform implementations must provide. This enables polymorphic usage
/// of different forge platforms through a unified API.
pub mod traits;

/// Common data types and structures used across forge platforms.
///
/// Contains shared data structures for releases, pull requests, repository
/// information, and other common concepts that exist across all supported
/// Git forge platforms.
pub mod types;
