//! Tests for the orchestrator module.
//!
//! Test organization:
//! - `common`: Shared test utilities and helper functions
//! - `metadata`: PR metadata parsing and regex matching tests
//! - `package_releases`: Package release creation tests
//! - `pr_workflow`: Pull request workflow tests
//! - `release_workflow`: Release creation workflow tests
//! - `next_release`: Next release workflow tests
//! - `current_releases`: Release data retrieval tests

mod common;
mod current_releases;
mod metadata;
mod next_release;
mod package_releases;
mod pr_workflow;
mod release_workflow;
