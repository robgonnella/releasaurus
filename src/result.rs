//! Unified error handling using `color-eyre` for enhanced error reporting.

use color_eyre::eyre::Result as EyreResult;

use crate::{analyzer::release::Release, config::ReleaseType};

/// Represents a release-able package in manifest
#[derive(Debug)]
pub struct ReleasablePackage {
    pub name: String,
    pub path: String,
    pub release_type: ReleaseType,
    pub release: Release,
}

/// Type alias for Result with color-eyre error reporting and diagnostics.
pub type Result<T> = EyreResult<T>;
