//! CLI command implementations for release automation workflow.
//!
//! Contains `release-pr` (create release pull request) and `release`
//! (publish final release) commands with shared utilities.

pub mod args;
pub mod common;
pub mod errors;
pub mod release;
pub mod release_pr;
pub mod types;
