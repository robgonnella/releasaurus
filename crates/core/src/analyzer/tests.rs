//! Tests for the analyzer module.
//!
//! Test organization:
//! - `basic_versioning`: Basic analyzer functionality (construction, version bumping)
//! - `filtering`: Commit filtering tests (skip_ci, skip_chore, etc.)
//! - `prerelease`: Prerelease versioning tests
//! - `rendering`: Default `body` template output and group ordering
//! - `version_rules`: Custom version increment rules and regex tests
//! - `version_types`: Version type tests (semantic+build, date-based)

mod basic_versioning;
mod filtering;
mod prerelease;
mod rendering;
mod version_rules;
mod version_types;

use chrono::Utc;

/// Runs `f` with today's date, retrying if the date changes mid-call.
pub(super) fn on_stable_date<T>(f: impl Fn(&str) -> T) -> (String, T) {
    loop {
        let day = Utc::now().format("%Y.%m.%d").to_string();
        let out = f(&day);
        if Utc::now().format("%Y.%m.%d").to_string() == day {
            return (day, out);
        }
    }
}
