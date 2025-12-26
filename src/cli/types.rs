use serde::{Serialize, ser::SerializeStruct};

use crate::{
    analyzer::release::Release, config::release_type::ReleaseType,
    updater::manager::ManifestFile,
};

/// Represents a release-able package in manifest
#[derive(Debug, Default)]
pub struct ReleasablePackage {
    /// The name of this package
    pub name: String,
    /// Path to package directory relative to workspace_root path
    pub path: String,
    /// Path to the workspace root directory for this package relative to the
    /// repository root
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

impl Serialize for ReleasablePackage {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("ReleasablePackage", 5)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("path", &self.path)?;
        s.serialize_field("workspace_root", &self.workspace_root)?;
        s.serialize_field("release_type", &self.release_type)?;
        s.serialize_field("release", &self.release)?;
        s.end()
    }
}
