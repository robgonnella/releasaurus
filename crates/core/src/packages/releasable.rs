use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    analyzer::release::Release, config::release_type::ReleaseType,
    forge::request::Tag, updater::manager::ManifestTarget,
};

pub type BranchName = String;

/// Releasable packages keyed by the release branch they share. Ordered
/// so a run's commits, PRs, and changelog sections come out the same way
/// every time — the packages feeding it arrive in `HashMap` order.
pub type ReleasablePackageGroups = BTreeMap<BranchName, Vec<ReleasablePackage>>;

/// A sub-package sharing its parent's release tag and changelog but
/// receiving its own independent manifest updates.
#[derive(Debug, Default, Clone, Serialize)]
pub struct ReleasableSubPackage {
    pub name: String,
    /// Normalized, repo-relative directory. Manifest ownership is
    /// derived from it.
    pub path: PathBuf,
    pub release_type: ReleaseType,
    pub manifest_targets: Vec<ManifestTarget>,
}

/// Package ready for manifest updates and PR creation, with a
/// computed next-version tag, changelog notes, and loaded manifest
/// file content.
#[derive(Debug, Default, Clone)]
pub struct ReleasablePackage {
    pub name: String,
    /// Normalized, repo-relative directory. Manifest ownership is
    /// derived from it.
    pub path: PathBuf,
    pub release_type: ReleaseType,
    pub tag: Tag,
    pub notes: String,
    pub tag_compare_link: String,
    pub sha_compare_link: String,
    pub sub_packages: Vec<ReleasableSubPackage>,
    pub manifest_targets: Vec<ManifestTarget>,
    pub additional_manifest_targets: Vec<(ManifestTarget, Regex)>,
}

/// Serializable form of a releasable package including full commit
/// history. Used for the `get next-release` command and for writing
/// release metadata to a file.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SerializableReleasablePackage {
    /// The name of this package
    pub name: String,
    pub path: PathBuf,
    pub release_type: ReleaseType,
    pub release: Release,
    #[serde(skip)]
    pub sub_packages: Vec<ReleasableSubPackage>,
    #[serde(skip)]
    pub manifest_targets: Vec<ManifestTarget>,
    #[serde(skip)]
    pub additional_manifest_targets: Vec<(ManifestTarget, Regex)>,
}
