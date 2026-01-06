use std::rc::Rc;

use serde::{Serialize, ser::SerializeStruct};

use crate::{
    analyzer::release::Release,
    config::release_type::ReleaseType,
    updater::manager::{AdditionalManifestFile, ManifestFile},
};

#[derive(Debug, Default, Clone)]
pub struct ReleasableSubPackage {
    /// The name of this sub-package
    pub name: String,
    /// Path to package directory relative to workspace_root path of the
    /// parent package
    pub path: String,
    /// The [`ReleaseType`] for this package
    pub release_type: ReleaseType,
    /// Manifest version files specific to this package's release-type
    pub manifest_files: Option<Vec<ManifestFile>>,
}

impl ReleasableSubPackage {
    pub fn to_releasable(
        &self,
        parent: &ReleasablePackage,
    ) -> ReleasablePackage {
        ReleasablePackage {
            name: self.name.clone(),
            path: self.path.clone(),
            release_type: self.release_type,
            manifest_files: self.manifest_files.clone(),
            workspace_root: parent.workspace_root.clone(),
            release: Rc::clone(&parent.release),
            ..Default::default()
        }
    }
}

/// Represents a release-able package in manifest
#[derive(Debug, Default, Clone)]
pub struct ReleasablePackage {
    /// The name of this package
    pub name: String,
    /// Path to package directory relative to workspace_root path
    pub path: String,
    /// Path to the workspace root directory for this package relative to the
    /// repository root
    pub workspace_root: String,
    /// Groups sub-packages under a single release. Each will share changelog,
    /// tag, and release, but will receive independent manifest version updates
    /// according to their type
    pub sub_packages: Vec<ReleasableSubPackage>,
    /// The [`ReleaseType`] for this package
    pub release_type: ReleaseType,
    /// The computed Release for this package (shared via Rc to avoid cloning)
    pub release: Rc<Release>,
    /// Manifest version files specific to this package's release-type
    pub manifest_files: Option<Vec<ManifestFile>>,
    /// Additional generic version manifest files to update
    pub additional_manifest_files: Option<Vec<AdditionalManifestFile>>,
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
        s.serialize_field("release", self.release.as_ref())?;
        s.end()
    }
}
