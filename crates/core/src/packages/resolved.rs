use regex::Regex;
use std::path::PathBuf;

use crate::{
    analyzer::config::AnalyzerConfig,
    config::{prerelease::PrereleaseConfig, release_type::ReleaseType},
};

/// Compiled version of AdditionalManifest with pre-compiled regex.
///
/// This is populated during config resolution to avoid repeated
/// regex compilation during manifest processing.
#[derive(Debug, Clone)]
pub struct CompiledAdditionalManifest {
    /// The path to the manifest file relative to package path
    pub path: PathBuf,
    /// The compiled regex to use to match and replace versions
    pub version_regex: Regex,
}

/// A fully resolved package configuration ready for processing.
///
/// This type represents a package after all configuration sources
/// have been merged and validated. All optional values have been
/// resolved to concrete values, paths have been normalized, and
/// complex configurations (like analyzer config) have been built.
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub normalized_workspace_root: PathBuf,
    pub normalized_full_path: PathBuf,
    pub release_type: ReleaseType,
    pub tag_prefix: String,
    pub sub_packages: Vec<ResolvedPackage>,
    pub prerelease: Option<PrereleaseConfig>,
    pub auto_start_next: bool,
    pub normalized_additional_paths: Vec<PathBuf>,
    pub compiled_additional_manifests: Vec<CompiledAdditionalManifest>,
    pub analyzer_config: AnalyzerConfig,
}
