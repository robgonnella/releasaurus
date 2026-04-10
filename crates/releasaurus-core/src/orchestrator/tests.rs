//! Tests for the orchestrator module.
//!
//! Test organization:
//! - `common`: Shared test utilities and helper functions
//! - `metadata`: New-format PR metadata regex tests
//! - `metadata_legacy`: Legacy PR metadata regex tests
//! - `package_releases`: Package release creation tests
//! - `pr_workflow`: Pull request workflow tests
//! - `release_workflow`: Release creation workflow tests
//! - `next_release`: Next release workflow tests
//! - `current_releases`: Release data retrieval tests
//! - `get_notes`: Get notes functionality tests

pub(crate) mod common;
mod current_releases;
mod get_notes;
mod metadata;
mod metadata_legacy;
mod next_release;
mod package_releases;
mod pr_workflow;
mod release_workflow;
