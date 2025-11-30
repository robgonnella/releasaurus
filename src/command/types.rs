use color_eyre::Result as EyreResult;

use crate::{
    analyzer::release::Release, config::release_type::ReleaseType,
    updater::manager::ManifestFile,
};

/// Type alias for Result with color-eyre error reporting and diagnostics.
pub type Result<T> = EyreResult<T>;

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
    /// Manifest version files specific to this package's release-type
    pub manifest_files: Option<Vec<ManifestFile>>,
    /// Additional generic version manifest files to update
    pub additional_manifest_files: Option<Vec<ManifestFile>>,
}
