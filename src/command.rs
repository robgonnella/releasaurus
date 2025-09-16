//! CLI command implementations for release automation workflow.
//!
//! Contains `release-pr` (create release pull request) and `release`
//! (publish final release) commands with shared utilities.

/// Shared utilities and common functionality for commands.
pub mod common;

/// Final release publication and tagging command implementation.
pub mod release;

/// Release PR creation and management command implementation.
pub mod release_pr;

/// End-to-end integration tests (requires `_internal_e2e_tests` feature).
#[cfg(test)]
#[cfg(feature = "_internal_e2e_tests")]
mod tests;
