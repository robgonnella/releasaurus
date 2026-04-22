//! Tests for the orchestrator core module.
//!
//! Test organization:
//! - `common`: Shared test utilities and helper functions
//! - `analyze`: Package analysis tests (analyzing commits, version bumping)
//! - `prepare`: Package preparation tests (dummy commits, target filtering)
//! - `pr_grouping`: PR grouping and branch logic tests (separate vs grouped)
//! - `pr_requests`: PR request generation and branch creation tests
//!   (metadata, file changes, branch creation order)

mod analyze;
mod common;
mod pr_grouping;
mod pr_requests;
mod prepare;
