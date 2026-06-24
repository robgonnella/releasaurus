//! Analyzer configuration resolution.
//!
//! Builds AnalyzerConfig instances from resolved package parameters,
//! handling complex interactions between global config, package
//! config, and CLI overrides.

use std::rc::Rc;

use crate::{
    analyzer::config::AnalyzerConfig,
    config::{prerelease::PrereleaseConfig, resolved::ResolvedConfig},
};

/// Parameters for building an analyzer configuration.
///
/// This is an internal type used to pass resolved values from
/// package configuration into the analyzer config builder.
#[derive(Debug)]
pub struct AnalyzerParams {
    pub config: Rc<ResolvedConfig>,
    pub package_name: String,
    pub prerelease: Option<PrereleaseConfig>,
    pub tag_prefix: String,
    pub breaking_always_increment_major: bool,
    pub custom_major_increment_regex: Option<String>,
    pub features_always_increment_minor: bool,
    pub custom_minor_increment_regex: Option<String>,
}

/// Builds an AnalyzerConfig from resolved parameters.
///
/// This function combines global configuration, package-specific
/// settings, and generates package-specific patterns (like release
/// commit matcher).
pub fn build_analyzer_config(params: AnalyzerParams) -> AnalyzerConfig {
    AnalyzerConfig {
        body: params.config.changelog.body.clone(),
        breaking_always_increment_major: params.breaking_always_increment_major,
        custom_major_increment_regex: params.custom_major_increment_regex,
        custom_minor_increment_regex: params.custom_minor_increment_regex,
        features_always_increment_minor: params.features_always_increment_minor,
        include_author: params.config.changelog.include_author,
        prerelease: params.prerelease,
        release_link_base_url: Some(
            params.config.release_link_base_url.clone(),
        ),
        compare_link_base_url: Some(
            params.config.compare_link_base_url.clone(),
        ),
        skip_merge_commits: params.config.changelog.skip_merge_commits,
        tag_prefix: Some(params.tag_prefix),
        commit_modifiers: params.config.commit_modifiers.clone(),
    }
}
