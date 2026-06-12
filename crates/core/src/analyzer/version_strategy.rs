//! Version strategy trait and implementations for calculating next versions.
//!
//! This module provides a trait-based approach to version calculation,
//! allowing different strategies for stable releases, versioned prereleases,
//! and static prereleases.

pub(crate) mod context;
pub(crate) mod date;
pub(crate) mod date_with_time;
pub(crate) mod date_with_time_micro;
pub(crate) mod factory;
pub(crate) mod prerelease_static;
pub(crate) mod prerelease_versioned;
pub(crate) mod semantic;
pub(crate) mod semantic_build;
pub(crate) mod traits;
