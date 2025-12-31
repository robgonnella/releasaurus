//! Configuration resolver using builder pattern.
//!
//! Transforms raw [`Config`] from `releasaurus.toml` into fully resolved configuration
//! by applying defaults, merging CLI overrides, and validating inputs.
//!
//! ## Resolution Precedence (highest to lowest)
//!
//! 1. Package-specific CLI overrides
//! 2. Global CLI overrides
//! 3. Package-level config from file
//! 4. Global config from file
//! 5. Repository defaults
//! 6. Built-in defaults

use derive_builder::Builder;
use regex::Regex;
use std::{collections::HashMap, path::Path};

use crate::{
    Result,
    analyzer::config::AnalyzerConfig,
    cli::{CommitModifiers, GlobalOverrides, PackageOverrides, validate_sha},
    config::{
        package::{
            CompiledAdditionalManifest, DEFAULT_TAG_PREFIX, PackageConfig,
        },
        prerelease::PrereleaseConfig,
    },
    error::ReleasaurusError,
};

use super::Config;

/// Resolves configuration by taking ownership and applying all resolution logic.
#[derive(Builder)]
#[builder(setter(into))]
pub struct ConfigResolver {
    config: Config,
    repo_name: String,
    repo_default_branch: String,
    release_link_base_url: String,
    package_overrides: HashMap<String, PackageOverrides>,
    global_overrides: GlobalOverrides,
    commit_modifiers: CommitModifiers,
}

impl ConfigResolver {
    /// Resolves the configuration and returns the fully resolved Config.
    pub fn resolve(&self) -> Result<Config> {
        let mut config = self.config.clone();

        Self::resolve_base_branch(
            &mut config,
            &self.global_overrides,
            &self.repo_default_branch,
        );
        let base_branch = config.base_branch.as_ref().unwrap().clone();
        let commit_modifiers = Self::resolve_commit_modifiers(
            &mut config,
            &self.commit_modifiers,
        )?;

        for package in config.packages.iter_mut() {
            Self::resolve_package_name(package, &self.repo_name);
            Self::resolve_tag_prefix(package);
            Self::resolve_auto_start_next(package, config.auto_start_next);
            Self::resolve_prerelease(
                package,
                &config.prerelease,
                &self.global_overrides,
                &self.package_overrides,
            )?;

            let release_commit_matcher =
                Self::build_release_commit_matcher(&base_branch, &package.name);

            let (
                breaking_always_increment_major,
                features_always_increment_minor,
            ) = Self::resolve_version_increment_flags(
                package,
                config.breaking_always_increment_major,
                config.features_always_increment_minor,
            );

            let (custom_major_increment_regex, custom_minor_increment_regex) =
                Self::resolve_custom_increment_regexes(
                    package,
                    &config.custom_major_increment_regex,
                    &config.custom_minor_increment_regex,
                );

            Self::compile_additional_manifests(package)?;

            package.analyzer_config = AnalyzerConfig {
                body: config.changelog.body.clone(),
                breaking_always_increment_major,
                custom_major_increment_regex,
                custom_minor_increment_regex,
                features_always_increment_minor,
                include_author: config.changelog.include_author,
                prerelease: package.prerelease.clone(),
                release_commit_matcher,
                release_link_base_url: self.release_link_base_url.clone(),
                skip_chore: config.changelog.skip_chore,
                skip_ci: config.changelog.skip_ci,
                skip_merge_commits: config.changelog.skip_merge_commits,
                skip_miscellaneous: config.changelog.skip_miscellaneous,
                skip_release_commits: config.changelog.skip_release_commits,
                tag_prefix: package.tag_prefix.clone(),
                commit_modifiers: commit_modifiers.clone(),
            };
        }

        Ok(config)
    }

    fn resolve_base_branch(
        config: &mut Config,
        global_overrides: &GlobalOverrides,
        repo_default_branch: &str,
    ) {
        config.base_branch = Some(
            global_overrides
                .base_branch
                .clone()
                .or_else(|| config.base_branch.take())
                .unwrap_or_else(|| repo_default_branch.to_string()),
        );
    }

    fn resolve_commit_modifiers(
        config: &mut Config,
        cli_modifiers: &CommitModifiers,
    ) -> Result<CommitModifiers> {
        let skip_shas = if !cli_modifiers.skip_shas.is_empty() {
            cli_modifiers.skip_shas.clone()
        } else if let Some(list) = config.changelog.skip_shas.take() {
            for sha in &list {
                validate_sha(sha).map_err(|e| {
                    ReleasaurusError::invalid_config(format!(
                        "Invalid SHA in changelog.skip_shas: {}",
                        e
                    ))
                })?;
            }
            list
        } else {
            Vec::new()
        };

        let reword = if !cli_modifiers.reword.is_empty() {
            cli_modifiers.reword.clone()
        } else if let Some(list) = config.changelog.reword.take() {
            for entry in &list {
                validate_sha(&entry.sha).map_err(|e| {
                    ReleasaurusError::invalid_config(format!(
                        "Invalid SHA in changelog.reword: {}",
                        e
                    ))
                })?;
            }
            list
        } else {
            Vec::new()
        };

        Ok(CommitModifiers { skip_shas, reword })
    }

    fn resolve_package_name(package: &mut PackageConfig, repo_name: &str) {
        if !package.name.is_empty() {
            return;
        }

        package.name = Path::new(&package.workspace_root)
            .join(&package.path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| repo_name.to_string());
    }

    fn resolve_tag_prefix(package: &mut PackageConfig) {
        if package.tag_prefix.is_some() {
            return;
        }

        package.tag_prefix =
            Some(if package.workspace_root != "." || package.path != "." {
                format!("{}-v", package.name)
            } else {
                DEFAULT_TAG_PREFIX.to_string()
            });
    }

    fn resolve_auto_start_next(
        package: &mut PackageConfig,
        global_auto_start_next: Option<bool>,
    ) {
        package.auto_start_next =
            package.auto_start_next.or(global_auto_start_next);
    }

    fn resolve_prerelease(
        package: &mut PackageConfig,
        global_prerelease: &PrereleaseConfig,
        global_overrides: &GlobalOverrides,
        package_overrides: &HashMap<String, PackageOverrides>,
    ) -> Result<()> {
        let mut prerelease = global_prerelease.clone();

        if let Some(pkg_prerelease) = package.prerelease.take() {
            prerelease = pkg_prerelease;
        }

        if let Some(ref suffix) = global_overrides.prerelease_suffix {
            prerelease.suffix = Some(suffix.clone());
        }
        if let Some(strategy) = global_overrides.prerelease_strategy {
            prerelease.strategy = strategy;
        }

        if let Some(overrides) = package_overrides.get(&package.name) {
            if let Some(ref suffix) = overrides.prerelease_suffix {
                prerelease.suffix = Some(suffix.clone());
            }
            if let Some(strategy) = overrides.prerelease_strategy {
                prerelease.strategy = strategy;
            }
        }

        prerelease.suffix = prerelease
            .suffix
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        package.prerelease = if prerelease.suffix.is_some() {
            Some(prerelease)
        } else {
            None
        };

        Ok(())
    }

    fn build_release_commit_matcher(
        base_branch: &str,
        package_name: &str,
    ) -> Option<Regex> {
        Regex::new(&format!(
            r#"^chore\({base_branch}\): release {package_name}"#
        ))
        .ok()
    }

    fn resolve_version_increment_flags(
        package: &PackageConfig,
        global_breaking: bool,
        global_features: bool,
    ) -> (bool, bool) {
        let breaking = package
            .breaking_always_increment_major
            .unwrap_or(global_breaking);
        let features = package
            .features_always_increment_minor
            .unwrap_or(global_features);
        (breaking, features)
    }

    fn resolve_custom_increment_regexes(
        package: &mut PackageConfig,
        global_custom_major: &Option<String>,
        global_custom_minor: &Option<String>,
    ) -> (Option<String>, Option<String>) {
        let major = package
            .custom_major_increment_regex
            .take()
            .or_else(|| global_custom_major.clone());
        let minor = package
            .custom_minor_increment_regex
            .take()
            .or_else(|| global_custom_minor.clone());
        (major, minor)
    }

    fn compile_additional_manifests(package: &mut PackageConfig) -> Result<()> {
        let Some(manifest_specs) = package.additional_manifest_files.take()
        else {
            package.compiled_additional_manifests = Vec::new();
            return Ok(());
        };

        let mut compiled = Vec::with_capacity(manifest_specs.len());

        for spec in manifest_specs {
            let manifest = spec.into_manifest();

            let pattern = manifest.version_regex.as_ref().ok_or_else(|| {
                ReleasaurusError::invalid_config(format!(
                    "Missing version_regex for additional_manifest_files entry '{}'. This should not happen after spec conversion.",
                    manifest.path
                ))
            })?;

            let version_regex = Regex::new(pattern).map_err(|e| {
                ReleasaurusError::invalid_config(format!(
                    "Invalid regex pattern in additional_manifest_files for '{}': {}",
                    manifest.path, e
                ))
            })?;

            // Validate that the regex has a 'version' capture group
            let has_version_group = version_regex
                .capture_names()
                .any(|name| name == Some("version"));

            if !has_version_group {
                return Err(ReleasaurusError::invalid_config(format!(
                    "Regex pattern for '{}' must include a named capture group '(?<version>...)' \
                     to identify the version number to replace",
                    manifest.path
                )));
            }

            compiled.push(CompiledAdditionalManifest {
                path: manifest.path,
                version_regex,
            });
        }

        package.compiled_additional_manifests = compiled;
        Ok(())
    }
}

#[cfg(test)]
#[path = "resolver_tests.rs"]
mod resolver_tests;
