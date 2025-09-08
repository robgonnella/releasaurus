//! Command execution and orchestration for Releasaurus.
//!
//! This module contains the implementation of all CLI commands available in Releasaurus.
//! Each command represents a different stage of the release automation workflow, from
//! preparation to final publication.
//!
//! # Architecture
//!
//! The command system is organized into distinct modules:
//!
//! - **common**: Shared functionality and utilities used across multiple commands
//! - **release_pr**: Prepare and create release pull requests
//! - **release**: Execute the final release publication process
//!
//! Each command module follows a consistent pattern:
//! 1. Parse and validate CLI arguments
//! 2. Initialize necessary services (Git, forge APIs, analyzers)
//! 3. Execute the command-specific workflow
//! 4. Handle errors and provide meaningful feedback
//!
//! # Command Workflow
//!
//! ## Release PR Command (`release_pr`)
//!
//! 1. **Analysis**: Analyze commits since the last release using git-cliff
//! 2. **Version Detection**: Determine the next semantic version based on commit types
//! 3. **File Updates**: Update version files across supported languages and frameworks
//! 4. **Changelog Generation**: Create or update changelog with new release information
//! 5. **PR Creation**: Commit changes and create a pull request for review
//!
//! ## Release Command (`release`)
//!
//! 1. **Validation**: Ensure we're on a proper release commit
//! 2. **Tagging**: Create Git tags for the release version
//! 3. **Publishing**: Push tags and create releases in the forge platform
//! 4. **Artifact Management**: Handle any associated release artifacts
//!
//! # Error Handling
//!
//! All commands use the unified error handling system provided by the `result` module,
//! ensuring consistent error reporting and user-friendly messages across different
//! failure scenarios.
//!
//! # Dry Run Support
//!
//! Commands support dry-run mode through the `--dry-run` CLI flag, allowing users
//! to preview changes without making any modifications to the repository or remote
//! services.

/// Common utilities and shared functionality used across multiple commands.
///
/// This module contains helper functions, shared data structures, and common
/// operations that are used by multiple command implementations. It helps
/// reduce code duplication and ensures consistent behavior across commands.
pub mod common;

/// Release pull request creation and management.
///
/// Implements the `release-pr` command which analyzes the repository state,
/// determines version updates, updates relevant files, and creates a pull
/// request containing all changes needed for a release.
///
/// This command is typically the first step in the release process, creating
/// a reviewable set of changes before the final release is published.
pub mod release;

/// Final release publication and tagging.
///
/// Implements the `release` command which handles the final steps of the
/// release process, including Git tag creation, pushing to remote repositories,
/// and creating releases in forge platforms (GitHub, GitLab, Gitea).
///
/// This command is typically run after a release PR has been reviewed and merged.
pub mod release_pr;

/// End-to-end integration tests for command functionality.
///
/// These tests are only compiled when the `_internal_e2e_tests` feature is enabled,
/// as they require more extensive setup and may interact with external services
/// or create temporary repositories for testing complete workflows.
#[cfg(test)]
#[cfg(feature = "_internal_e2e_tests")]
mod tests;
