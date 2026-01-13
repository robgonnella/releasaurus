use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    analyzer::release::{Release, Tag},
    config::release_type::ReleaseType,
    updater::manager::{AdditionalManifestFile, ManifestFile},
};

#[derive(Debug, Default, Clone, Serialize)]
pub struct ReleasableSubPackage {
    pub name: String,
    pub release_type: ReleaseType,
    pub manifest_files: Option<Vec<ManifestFile>>,
}

impl ReleasableSubPackage {
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

/// Represents a release-able package complete with next version tag and loaded
/// manifest content
#[derive(Debug, Default)]
pub struct ReleasablePackage {
    pub name: String,
    pub release_type: ReleaseType,
    pub tag: Tag,
    pub notes: String,
    pub sub_packages: Vec<ReleasableSubPackage>,
    pub manifest_files: Option<Vec<ManifestFile>>,
    pub additional_manifest_files: Option<Vec<AdditionalManifestFile>>,
}

/// Represents a full serializable release-able package complete with
/// entire analyzed commit history
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
