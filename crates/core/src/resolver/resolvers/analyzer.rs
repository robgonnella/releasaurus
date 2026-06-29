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
        skip_chore: params.config.changelog.skip_chore,
        skip_ci: params.config.changelog.skip_ci,
        skip_doc: params.config.changelog.skip_doc,
        skip_perf: params.config.changelog.skip_perf,
        skip_test: params.config.changelog.skip_test,
        skip_refactor: params.config.changelog.skip_refactor,
        skip_revert: params.config.changelog.skip_revert,
        skip_style: params.config.changelog.skip_style,
        skip_merge_commits: params.config.changelog.skip_merge_commits,
        skip_miscellaneous: params.config.changelog.skip_miscellaneous,
        tag_prefix: Some(params.tag_prefix),
        commit_modifiers: params.config.commit_modifiers.clone(),
    }
}
