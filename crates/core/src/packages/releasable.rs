use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    analyzer::release::Release,
    config::release_type::ReleaseType,
    forge::request::Tag,
    packages::manifests::{AdditionalManifestFile, ManifestFile},
};

/// A sub-package sharing its parent's release tag and changelog but
/// receiving its own independent manifest updates.
#[derive(Debug, Default, Clone, Serialize)]
pub struct ReleasableSubPackage {
    pub name: String,
    pub release_type: ReleaseType,
    pub manifest_files: Option<Vec<ManifestFile>>,
}

impl ReleasableSubPackage {
    /// Build a `ReleasablePackage` from this sub-package, inheriting
    /// the parent's tag and release information.
    pub fn to_releasable(
        &self,
        parent: &ReleasablePackage,
    ) -> ReleasablePackage {
        ReleasablePackage {
            name: self.name.clone(),
            release_type: self.release_type,
            tag: parent.tag.clone(),
            manifest_files: self.manifest_files.clone(),
            ..Default::default()
        }
    }
}

/// Package ready for manifest updates and PR creation, with a
/// computed next-version tag, changelog notes, and loaded manifest
/// file content.
#[derive(Debug, Default)]
pub struct ReleasablePackage {
    pub name: String,
    pub release_type: ReleaseType,
    pub tag: Tag,
    pub notes: String,
    pub tag_compare_link: String,
    pub sha_compare_link: String,
    pub sub_packages: Vec<ReleasableSubPackage>,
    pub manifest_files: Option<Vec<ManifestFile>>,
    pub additional_manifest_files: Option<Vec<AdditionalManifestFile>>,
}

/// Serializable form of a releasable package including full commit
/// history. Used for the `get next-release` command and for writing
/// release metadata to a file.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SerializableReleasablePackage {
    /// The name of this package
    pub name: String,
    pub path: PathBuf,
    pub release_type: ReleaseType,
    pub release: Release,
    #[serde(skip)]
    pub sub_packages: Vec<ReleasableSubPackage>,
    #[serde(skip)]
    pub manifest_files: Option<Vec<ManifestFile>>,
    #[serde(skip)]
    pub additional_manifest_files: Option<Vec<AdditionalManifestFile>>,
}
