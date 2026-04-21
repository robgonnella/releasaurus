use regex::Regex;
use serde::{Serialize, ser::SerializeStruct};
use std::path::PathBuf;

use crate::config::package::GENERIC_VERSION_REGEX;

#[derive(Clone)]
pub struct AdditionalManifestFile {
    /// The file path relative to the package path
    pub path: PathBuf,
    /// The base name of the file path
    pub basename: String,
    /// The current content of the file
    pub content: String,
    /// The version regex to use to match and replace version content
    pub version_regex: Regex,
}

impl Default for AdditionalManifestFile {
    fn default() -> Self {
        Self {
            path: "".into(),
            basename: "".into(),
            content: "".into(),
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

#[derive(Default, Clone)]
pub struct ManifestFile {
    /// The file path relative to the package path
    pub path: PathBuf,
    /// The base name of the file path
    pub basename: String,
    /// The current content of the file
    pub content: String,
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
        }
    }
}

impl From<&AdditionalManifestFile> for ManifestFile {
    fn from(value: &AdditionalManifestFile) -> Self {
        Self {
            path: value.path.clone(),
            basename: value.basename.clone(),
            content: value.content.clone(),
        }
    }
}
