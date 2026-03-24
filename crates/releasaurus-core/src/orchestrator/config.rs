use derive_builder::Builder;
use merge::Merge;
use std::{collections::HashMap, rc::Rc};
use url::Url;

use crate::{
    config::{
        Config,
        changelog::{ChangelogConfig, RewordedCommit},
        prerelease::{PrereleaseConfig, PrereleaseStrategy},
    },
    error::{ReleasaurusError, Result},
};

/// Validates that a string is a valid git commit SHA (7-40 hex characters)
pub fn validate_sha(sha: &str) -> Result<String> {
    let trimmed = sha.trim();

    if trimmed.len() < 7 {
        return Err(ReleasaurusError::invalid_config(format!(
            "Invalid commit SHA: '{}'. Must be at least 7 characters",
            sha
        )));
    }

    if trimmed.len() > 40 {
        return Err(ReleasaurusError::invalid_config(format!(
            "Invalid commit SHA: '{}'. Must not exceed 40 characters",
            sha
        )));
    }

    if !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ReleasaurusError::invalid_config(format!(
            "Invalid commit SHA: '{}'. Must contain only hexadecimal characters (0-9, a-f)",
            sha
        )));
    }

    Ok(trimmed.to_string())
}

#[derive(Debug, Clone, Merge)]
pub struct PackageOverrides {
    #[merge(strategy = merge::option::overwrite_none)]
    pub tag_prefix: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub prerelease_suffix: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub prerelease_strategy: Option<PrereleaseStrategy>,
}

#[derive(Debug, Clone, Default, Merge)]
pub struct GlobalOverrides {
    #[merge(strategy = merge::option::overwrite_none)]
    pub base_branch: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub tag_prefix: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub prerelease_suffix: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub prerelease_strategy: Option<PrereleaseStrategy>,
}

#[derive(Debug, Clone, Default)]
pub struct CommitModifiers {
    /// Commit sha (or prefix) to skip when calculating next version and
    /// generating changelog. Matches any commit whose SHA starts with the
    /// provided value
    pub skip_shas: Vec<String>,
    /// Rewords a commit message when generating changelog. The SHA can be a
    /// prefix - matches any commit whose SHA starts with the provided value.
    pub reword: Vec<RewordedCommit>,
}

#[derive(Debug, Builder)]
#[builder(setter(into), build_fn(private, name = "_build"))]
pub struct OrchestratorConfigParams {
    pub toml_config: Rc<Config>,
    pub repo_name: String,
    pub repo_default_branch: String,
    pub release_link_base_url: Url,
    pub compare_link_base_url: Url,
    pub package_overrides: HashMap<String, PackageOverrides>,
    pub global_overrides: GlobalOverrides,
    pub commit_modifiers: CommitModifiers,
}

impl OrchestratorConfigParamsBuilder {
    pub fn build(&self) -> Result<OrchestratorConfig> {
        let params = self._build().map_err(|e| {
            ReleasaurusError::invalid_config(format!(
                "Failed to build core config: {}",
                e
            ))
        })?;
        OrchestratorConfig::new(params)
    }
}

#[derive(Debug)]
pub struct OrchestratorConfig {
    pub repo_name: String,
    pub base_branch: String,
    pub release_link_base_url: Url,
    pub compare_link_base_url: Url,
    pub package_overrides: HashMap<String, PackageOverrides>,
    pub global_overrides: GlobalOverrides,
    pub commit_modifiers: CommitModifiers,
    pub first_release_search_depth: u64,
    pub separate_pull_requests: bool,
    pub prerelease: PrereleaseConfig,
    pub auto_start_next: Option<bool>,
    pub breaking_always_increment_major: bool,
    pub features_always_increment_minor: bool,
    pub custom_major_increment_regex: Option<String>,
    pub custom_minor_increment_regex: Option<String>,
    pub changelog: ChangelogConfig,
}

impl OrchestratorConfig {
    pub fn builder() -> OrchestratorConfigParamsBuilder {
        OrchestratorConfigParamsBuilder::default()
    }

    pub fn new(params: OrchestratorConfigParams) -> Result<Self> {
        let base_branch = Self::resolve_base_branch(
            &params.toml_config,
            &params.global_overrides,
            &params.repo_default_branch,
        );

        let commit_modifiers = Self::resolve_commit_modifiers(
            &params.toml_config,
            &params.commit_modifiers,
        )?;

        Ok(Self {
            auto_start_next: params.toml_config.auto_start_next,
            base_branch,
            breaking_always_increment_major: params
                .toml_config
                .breaking_always_increment_major,
            changelog: params.toml_config.changelog.clone(),
            commit_modifiers,
            custom_major_increment_regex: params
                .toml_config
                .custom_major_increment_regex
                .clone(),
            custom_minor_increment_regex: params
                .toml_config
                .custom_minor_increment_regex
                .clone(),
            features_always_increment_minor: params
                .toml_config
                .features_always_increment_minor,
            first_release_search_depth: params
                .toml_config
                .first_release_search_depth,
            global_overrides: params.global_overrides,
            package_overrides: params.package_overrides,
            prerelease: params.toml_config.prerelease.clone(),
            release_link_base_url: params.release_link_base_url,
            compare_link_base_url: params.compare_link_base_url,
            repo_name: params.repo_name,
            separate_pull_requests: params.toml_config.separate_pull_requests,
        })
    }

    fn resolve_base_branch(
        config: &Config,
        global_overrides: &GlobalOverrides,
        repo_default_branch: &str,
    ) -> String {
        global_overrides
            .base_branch
            .clone()
            .or_else(|| config.base_branch.clone())
            .unwrap_or_else(|| repo_default_branch.to_string())
    }

    fn resolve_commit_modifiers(
        config: &Config,
        modifiers: &CommitModifiers,
    ) -> Result<CommitModifiers> {
        let skip_shas = if !modifiers.skip_shas.is_empty() {
            modifiers.skip_shas.clone()
        } else if let Some(list) = config.changelog.skip_shas.clone() {
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

        let reword = if !modifiers.reword.is_empty() {
            modifiers.reword.clone()
        } else if let Some(list) = config.changelog.reword.clone() {
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
}
