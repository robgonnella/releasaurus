use derive_builder::Builder;
use std::{collections::HashMap, rc::Rc};
use url::Url;

use crate::{
    ReleasaurusError, Result,
    cli::{CommitModifiers, GlobalOverrides, PackageOverrides, validate_sha},
    config::{
        Config, changelog::ChangelogConfig, prerelease::PrereleaseConfig,
    },
};

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
        cli_modifiers: &CommitModifiers,
    ) -> Result<CommitModifiers> {
        let skip_shas = if !cli_modifiers.skip_shas.is_empty() {
            cli_modifiers.skip_shas.clone()
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

        let reword = if !cli_modifiers.reword.is_empty() {
            cli_modifiers.reword.clone()
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
