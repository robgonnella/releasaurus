//! Version strategy trait and implementations for calculating next versions.
//!
//! This module provides a trait-based approach to version calculation,
//! allowing different strategies for stable releases, versioned prereleases,
//! and static prereleases.

pub mod context;
pub mod date;
pub mod date_with_time;
pub mod date_with_time_micro;
pub mod factory;
pub mod prerelease_static;
pub mod prerelease_versioned;
pub mod semantic;
pub mod semantic_build;
pub mod traits;
