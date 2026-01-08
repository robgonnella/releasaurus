//! Resolved package configuration.
//!
//! This module handles the resolution of package configuration from
//! multiple sources (TOML config, CLI overrides, defaults) into a
//! single, validated ResolvedPackage ready for processing.

use derive_builder::Builder;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::{
    OrchestratorConfig, ReleasaurusError, Result,
    analyzer::config::AnalyzerConfig,
    config::{
        package::PackageConfig, prerelease::PrereleaseConfig,
        release_type::ReleaseType,
    },
};

pub mod analyzer;
pub mod hash;
pub mod manifest;
pub mod resolvers;

pub use hash::ResolvedPackageHash;
pub use manifest::CompiledAdditionalManifest;

use analyzer::{AnalyzerParams, build_analyzer_config};
use manifest::compile_additional_manifests;
use resolvers::*;

use super::path_utils::normalize_path;

/// Builder parameters for constructing a ResolvedPackage.
#[derive(Debug, Builder)]
#[builder(setter(into), build_fn(private, name = "_build"))]
pub struct ResolvedPackageParams {
    orchestrator_config: Rc<OrchestratorConfig>,
    package_config: PackageConfig,
}

impl ResolvedPackageParamsBuilder {
    pub fn build(&self) -> Result<ResolvedPackage> {
        let params = self._build().map_err(|e| {
            ReleasaurusError::invalid_config(format!(
                "Failed to build resolved package: {}",
                e
            ))
        })?;
        ResolvedPackage::new(params)
    }
}

/// A fully resolved package configuration ready for processing.
///
/// This type represents a package after all configuration sources
/// have been merged and validated. All optional values have been
/// resolved to concrete values, paths have been normalized, and
/// complex configurations (like analyzer config) have been built.
#[derive(Debug)]
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

impl ResolvedPackage {
    /// Creates a builder for constructing a ResolvedPackage.
    pub fn builder() -> ResolvedPackageParamsBuilder {
        ResolvedPackageParamsBuilder::default()
    }

    /// Constructs a new ResolvedPackage from builder parameters.
    ///
    /// This orchestrates all the resolution logic by calling
    /// various resolver functions in the correct order.
    pub fn new(params: ResolvedPackageParams) -> Result<Self> {
        // Resolve basic properties
        let name = resolve_package_name(
            &params.package_config,
            &params.orchestrator_config.repo_name,
        );
        let tag_prefix = resolve_tag_prefix(&params.package_config);
        let auto_start = resolve_auto_start_next(
            &params.package_config,
            params.orchestrator_config.auto_start_next,
        );

        // Resolve complex configurations
        let prerelease = resolve_prerelease(
            &params.package_config,
            &params.orchestrator_config.prerelease,
            &params.orchestrator_config.global_overrides,
            &params.orchestrator_config.package_overrides,
        )?;

        let (breaking_always_increment_major, features_always_increment_minor) =
            resolve_version_increment_flags(
                &params.package_config,
                params.orchestrator_config.breaking_always_increment_major,
                params.orchestrator_config.features_always_increment_minor,
            );

        let (custom_major_increment_regex, custom_minor_increment_regex) =
            resolve_custom_increment_regexes(
                &params.package_config,
                &params.orchestrator_config.custom_major_increment_regex,
                &params.orchestrator_config.custom_minor_increment_regex,
            );

        // Normalize paths
        let (normalized_workspace_root, normalized_full_path) =
            normalize_package_paths(&params.package_config);

        // Compile manifests
        let compiled_additional_manifests = compile_additional_manifests(
            &normalized_full_path,
            &params.package_config,
        )?;

        // Resolve additional paths
        let normalized_additional_paths =
            normalize_additional_paths(&params.package_config);

        // Build analyzer config
        let analyzer_config = build_analyzer_config(AnalyzerParams {
            config: Rc::clone(&params.orchestrator_config),
            package_name: name.clone(),
            prerelease: prerelease.clone(),
            tag_prefix: tag_prefix.clone(),
            breaking_always_increment_major,
            custom_major_increment_regex,
            features_always_increment_minor,
            custom_minor_increment_regex,
        });

        // Resolve sub-packages
        let sub_packages = resolve_sub_packages_full(
            &params,
            &normalized_workspace_root,
            &tag_prefix,
            prerelease.clone(),
            auto_start,
            &analyzer_config,
        );

        Ok(ResolvedPackage {
            name,
            normalized_workspace_root,
            normalized_full_path,
            release_type: params
                .package_config
                .release_type
                .unwrap_or_default(),
            tag_prefix,
            sub_packages,
            prerelease,
            auto_start_next: auto_start,
            normalized_additional_paths,
            compiled_additional_manifests,
            analyzer_config,
        })
    }
}

// Private helper functions

/// Normalizes workspace and full package paths.
fn normalize_package_paths(package: &PackageConfig) -> (PathBuf, PathBuf) {
    let mut normalized_root = normalize_path(&package.workspace_root);
    if normalized_root == "." {
        normalized_root = Cow::from("");
    }

    let normalized_workspace_root =
        Path::new(normalized_root.as_ref()).to_path_buf();

    let full_path = normalized_workspace_root
        .join(&package.path)
        .to_string_lossy()
        .to_string();

    let mut normalized_full = normalize_path(&full_path);
    if normalized_full == "." {
        normalized_full = Cow::from("");
    }

    let normalized_full_path =
        Path::new(normalized_full.as_ref()).to_path_buf();

    (normalized_workspace_root, normalized_full_path)
}

/// Normalizes additional paths for a package.
fn normalize_additional_paths(package: &PackageConfig) -> Vec<PathBuf> {
    package
        .additional_paths
        .clone()
        .unwrap_or_default()
        .iter()
        .map(|p| {
            let normalized = normalize_path(p).to_string();
            if normalized == "." {
                Path::new("").to_path_buf()
            } else {
                Path::new(&normalized).to_path_buf()
            }
        })
        .collect()
}

/// Resolves all sub-packages for a package.
fn resolve_sub_packages_full(
    params: &ResolvedPackageParams,
    normalized_workspace_root: &Path,
    tag_prefix: &str,
    prerelease: Option<PrereleaseConfig>,
    auto_start: bool,
    analyzer_config: &AnalyzerConfig,
) -> Vec<ResolvedPackage> {
    let sub_packages = params
        .package_config
        .sub_packages
        .clone()
        .unwrap_or_default();

    sub_packages
        .iter()
        .map(|s| {
            let name = resolve_sub_package_name(
                s,
                &params.package_config.workspace_root,
                &params.orchestrator_config.repo_name,
            );

            let sub_path = normalized_workspace_root
                .join(&s.path)
                .to_string_lossy()
                .to_string();

            let normalized_sub_full = normalize_path(&sub_path);
            let normalized_sub_full_path =
                Path::new(normalized_sub_full.as_ref()).to_path_buf();

            ResolvedPackage {
                name,
                normalized_workspace_root: normalized_workspace_root
                    .to_path_buf(),
                normalized_full_path: normalized_sub_full_path,
                release_type: s.release_type.unwrap_or_default(),
                tag_prefix: tag_prefix.to_string(),
                sub_packages: vec![],
                prerelease: prerelease.clone(),
                auto_start_next: auto_start,
                normalized_additional_paths: vec![],
                compiled_additional_manifests: vec![],
                analyzer_config: analyzer_config.clone(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        OrchestratorConfig,
        cli::{CommitModifiers, GlobalOverrides},
        config::package::{PackageConfigBuilder, SubPackage},
    };
    use std::rc::Rc;

    #[test]
    fn resolves_sub_packages_with_explicit_names() {
        let config = Rc::new(
            OrchestratorConfig::builder()
                .toml_config(Rc::new(Default::default()))
                .repo_name("test-repo")
                .repo_default_branch("main")
                .release_link_base_url("https://example.com")
                .package_overrides(std::collections::HashMap::new())
                .global_overrides(GlobalOverrides::default())
                .commit_modifiers(CommitModifiers::default())
                .build()
                .unwrap(),
        );

        let pkg_config = PackageConfigBuilder::default()
            .name("parent-pkg")
            .path(".")
            .sub_packages(vec![
                SubPackage {
                    name: "sub-pkg-a".to_string(),
                    path: "packages/a".to_string(),
                    ..Default::default()
                },
                SubPackage {
                    name: "sub-pkg-b".to_string(),
                    path: "packages/b".to_string(),
                    ..Default::default()
                },
            ])
            .build()
            .unwrap();

        let resolved = ResolvedPackage::builder()
            .orchestrator_config(config)
            .package_config(pkg_config)
            .build()
            .unwrap();

        assert_eq!(resolved.sub_packages.len(), 2);
        assert_eq!(resolved.sub_packages[0].name, "sub-pkg-a");
        assert_eq!(resolved.sub_packages[1].name, "sub-pkg-b");
    }

    #[test]
    fn resolves_sub_packages_with_auto_generated_names() {
        let config = Rc::new(
            OrchestratorConfig::builder()
                .toml_config(Rc::new(Default::default()))
                .repo_name("test-repo")
                .repo_default_branch("main")
                .release_link_base_url("https://example.com")
                .package_overrides(std::collections::HashMap::new())
                .global_overrides(GlobalOverrides::default())
                .commit_modifiers(CommitModifiers::default())
                .build()
                .unwrap(),
        );

        let pkg_config = PackageConfigBuilder::default()
            .name("parent-pkg")
            .path(".")
            .sub_packages(vec![SubPackage {
                name: "".to_string(),
                path: "packages/my-package".to_string(),
                ..Default::default()
            }])
            .build()
            .unwrap();

        let resolved = ResolvedPackage::builder()
            .orchestrator_config(config)
            .package_config(pkg_config)
            .build()
            .unwrap();

        assert_eq!(resolved.sub_packages.len(), 1);
        // Name should be derived from the last path component
        assert_eq!(resolved.sub_packages[0].name, "my-package");
    }

    #[test]
    fn sub_packages_inherit_parent_tag_prefix() {
        let config = Rc::new(
            OrchestratorConfig::builder()
                .toml_config(Rc::new(Default::default()))
                .repo_name("test-repo")
                .repo_default_branch("main")
                .release_link_base_url("https://example.com")
                .package_overrides(std::collections::HashMap::new())
                .global_overrides(GlobalOverrides::default())
                .commit_modifiers(CommitModifiers::default())
                .build()
                .unwrap(),
        );

        let pkg_config = PackageConfigBuilder::default()
            .name("parent-pkg")
            .path(".")
            .tag_prefix("v")
            .sub_packages(vec![SubPackage {
                name: "sub-pkg".to_string(),
                path: "packages/sub".to_string(),
                ..Default::default()
            }])
            .build()
            .unwrap();

        let resolved = ResolvedPackage::builder()
            .orchestrator_config(config)
            .package_config(pkg_config)
            .build()
            .unwrap();

        // Parent has explicit tag prefix
        assert_eq!(resolved.tag_prefix, "v");
        // Sub-packages should inherit the same tag prefix
        assert_eq!(resolved.sub_packages[0].tag_prefix, "v");
    }

    #[test]
    fn sub_packages_normalize_paths_correctly() {
        let config = Rc::new(
            OrchestratorConfig::builder()
                .toml_config(Rc::new(Default::default()))
                .repo_name("test-repo")
                .repo_default_branch("main")
                .release_link_base_url("https://example.com")
                .package_overrides(std::collections::HashMap::new())
                .global_overrides(GlobalOverrides::default())
                .commit_modifiers(CommitModifiers::default())
                .build()
                .unwrap(),
        );

        let pkg_config = PackageConfigBuilder::default()
            .name("parent-pkg")
            .path("workspace")
            .sub_packages(vec![SubPackage {
                name: "sub-pkg".to_string(),
                path: "packages/sub".to_string(),
                ..Default::default()
            }])
            .build()
            .unwrap();

        let resolved = ResolvedPackage::builder()
            .orchestrator_config(config)
            .package_config(pkg_config)
            .build()
            .unwrap();

        assert_eq!(resolved.sub_packages.len(), 1);
        // Path should contain the sub-package directory
        let sub_path_str = resolved.sub_packages[0]
            .normalized_full_path
            .to_string_lossy()
            .to_string();
        assert!(
            sub_path_str.contains("packages") || sub_path_str.contains("sub")
        );
        // Workspace root should match parent's workspace root
        assert_eq!(
            resolved.sub_packages[0].normalized_workspace_root,
            resolved.normalized_workspace_root
        );
    }

    #[test]
    fn handles_empty_sub_packages_list() {
        let config = Rc::new(
            OrchestratorConfig::builder()
                .toml_config(Rc::new(Default::default()))
                .repo_name("test-repo")
                .repo_default_branch("main")
                .release_link_base_url("https://example.com")
                .package_overrides(std::collections::HashMap::new())
                .global_overrides(GlobalOverrides::default())
                .commit_modifiers(CommitModifiers::default())
                .build()
                .unwrap(),
        );

        let pkg_config = PackageConfigBuilder::default()
            .name("parent-pkg")
            .path(".")
            .build()
            .unwrap();

        let resolved = ResolvedPackage::builder()
            .orchestrator_config(config)
            .package_config(pkg_config)
            .build()
            .unwrap();

        // Should have no sub-packages
        assert_eq!(resolved.sub_packages.len(), 0);
    }
}
