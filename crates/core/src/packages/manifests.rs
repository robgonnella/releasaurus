use regex::Regex;
use serde::{Serialize, ser::SerializeStruct};
use std::path::PathBuf;

use crate::{
    config::{package::GENERIC_VERSION_REGEX, release_type::ReleaseType},
    forge::request::Tag,
};

#[derive(Clone)]
pub struct AdditionalManifestFile {
    /// The file path relative to the package path
    pub path: PathBuf,
    /// The base name of the file path
    pub basename: String,
    /// The current content of the file
    pub content: String,
    /// Package that owns this file
    pub owner: ManifestPackage,
    /// The version regex to use to match and replace version content
    pub version_regex: Regex,
}

impl Default for AdditionalManifestFile {
    fn default() -> Self {
        Self {
            path: "".into(),
            basename: "".into(),
            content: "".into(),
            owner: ManifestPackage::default(),
            version_regex: GENERIC_VERSION_REGEX.clone(),
        }
    }
}

impl Serialize for AdditionalManifestFile {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("AdditionalManifestFile", 3)?;
        s.serialize_field("path", &self.path)?;
        s.serialize_field("basename", &self.basename)?;
        s.serialize_field("version_regex", &self.version_regex.as_str())?;
        s.end()
    }
}

impl std::fmt::Debug for AdditionalManifestFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdditionalManifestFile")
            .field("path", &self.path)
            .field("basename", &self.basename)
            .field("version_regex", &self.version_regex)
            .finish()
    }
}

#[derive(Default, Clone, Serialize)]
pub struct ManifestPackage {
    pub name: String,
    pub tag: Tag,
    pub release_type: ReleaseType,
}

#[derive(Default, Clone)]
pub struct ManifestFile {
    /// The file path relative to the package path
    pub path: PathBuf,
    /// The base name of the file path
    pub basename: String,
    /// The current content of the file
    pub content: String,
    /// Which language's updaters handle this file. Carried on the file
    /// rather than read off the owner, because a workspace-level file
    /// may have no owner at all.
    pub release_type: ReleaseType,
    /// The package this file's own version fields describe: the one
    /// whose directory it sits in.
    ///
    /// `None` for a file no releasing package owns — a virtual
    /// workspace manifest, or a root lock file in a workspace whose root
    /// is not being released. The absence *is* the condition that
    /// suppresses "write my own version"; there is no separate flag.
    pub owner: Option<ManifestPackage>,
    /// Every package being released alongside this file, of the same
    /// release type — **including** the owner.
    ///
    /// Owner-inclusive so that output cannot depend on which package
    /// drove the pass: there is no driver.
    pub releasing: Vec<ManifestPackage>,
}

impl Serialize for ManifestFile {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("ManifestFile", 2)?;
        s.serialize_field("path", &self.path)?;
        s.serialize_field("basename", &self.basename)?;
        s.end()
    }
}

impl std::fmt::Debug for ManifestFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManifestFile")
            .field("path", &self.path)
            .field("basename", &self.basename)
            .finish()
    }
}

impl From<AdditionalManifestFile> for ManifestFile {
    fn from(value: AdditionalManifestFile) -> Self {
        Self {
            path: value.path,
            basename: value.basename,
            content: value.content,
            release_type: value.owner.release_type,
            releasing: vec![value.owner.clone()],
            owner: Some(value.owner),
        }
    }
}

impl From<&AdditionalManifestFile> for ManifestFile {
    fn from(value: &AdditionalManifestFile) -> Self {
        value.clone().into()
    }
}
