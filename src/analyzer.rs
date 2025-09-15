//! Commit analysis, version detection, and changelog generation.
//!
//! Parses conventional commits, determines semantic version bumps,
//! and generates formatted changelogs using Tera templates.
pub mod changelog;
mod commit;
pub mod config;
mod groups;
mod helpers;
pub mod types;
