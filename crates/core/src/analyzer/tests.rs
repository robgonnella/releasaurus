//! Tests for the analyzer module.
//!
//! Test organization:
//! - `basic_versioning`: Basic analyzer functionality (construction, version bumping)
//! - `filtering`: Commit filtering tests (skip_ci, skip_chore, etc.)
//! - `prerelease`: Prerelease versioning tests
//! - `version_rules`: Custom version increment rules and regex tests

mod basic_versioning;
mod filtering;
mod prerelease;
mod version_rules;
