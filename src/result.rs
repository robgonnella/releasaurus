//! Unified error handling using `color-eyre` for enhanced error reporting.

use color_eyre::eyre::Result as EyreResult;
use std::fmt;

use crate::{analyzer::release::Release, config::ReleaseType};

/// Represents a release-able package in manifest
#[derive(Debug)]
pub struct ReleasablePackage {
    /// The name of this package
    pub name: String,
    /// Path to package directory relative to workspace_root path
    pub path: String,
    /// Path to the workspace root directory for this package relative to the repository root
    pub workspace_root: String,
    /// The [`ReleaseType`] for this package
    pub release_type: ReleaseType,
    /// The computed Release for this package
    pub release: Release,
}

/// Error indicating a pending release that hasn't been tagged yet.
///
/// This error is returned when attempting to create a new release PR
/// while a previous release PR has been merged but not yet tagged.
#[derive(Debug, Clone)]
pub struct PendingReleaseError {
    /// The release branch that has a pending release
    pub branch: String,
    /// The PR number of the pending release
    pub pr_number: u64,
}

impl fmt::Display for PendingReleaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "found pending release (PR #{}) on branch '{}' that has not been tagged yet: \
             cannot continue, must finish previous release first",
            self.pr_number, self.branch
        )
    }
}

impl std::error::Error for PendingReleaseError {}

/// Type alias for Result with color-eyre error reporting and diagnostics.
pub type Result<T> = EyreResult<T>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_release_error_into_eyre() {
        let error = PendingReleaseError {
            branch: "test-branch".to_string(),
            pr_number: 55,
        };

        // Test that it can be converted into color_eyre::eyre::Error
        let eyre_error: color_eyre::eyre::Error = error.into();
        let error_string = format!("{}", eyre_error);
        assert!(error_string.contains("PR #55"));
        assert!(error_string.contains("test-branch"));
    }
}
