//! Analyzer configuration resolution.
//!
//! Builds AnalyzerConfig instances from resolved package parameters,
//! handling complex interactions between global config, package
//! config, and CLI overrides.

use url::Url;

use crate::{
    analyzer::config::AnalyzerConfig,
    config::{
        changelog::{
            ChangelogConfig, DEFAULT_BODY, DEFAULT_INCLUDE_AUTHOR,
            DEFAULT_SKIP_MERGE_COMMITS,
        },
        prerelease::PrereleaseConfig,
        resolved::CommitModifiers,
    },
};

/// Parameters for building an analyzer configuration.
///
/// This is an internal type used to pass resolved values from
/// package configuration into the analyzer config builder.
#[derive(Debug)]
pub struct AnalyzerParams {
    pub changelog_config: ChangelogConfig,
    pub package_name: String,
    pub prerelease: Option<PrereleaseConfig>,
    pub tag_prefix: String,
    pub release_link_base_url: Option<Url>,
    pub compare_link_base_url: Option<Url>,
    pub commit_modifiers: CommitModifiers,
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
        body: params
            .changelog_config
            .body
            .clone()
            .unwrap_or(DEFAULT_BODY.into()),
        breaking_always_increment_major: params.breaking_always_increment_major,
        custom_major_increment_regex: params.custom_major_increment_regex,
        custom_minor_increment_regex: params.custom_minor_increment_regex,
        features_always_increment_minor: params.features_always_increment_minor,
        include_author: params
            .changelog_config
            .include_author
            .unwrap_or(DEFAULT_INCLUDE_AUTHOR),
        prerelease: params.prerelease,
        release_link_base_url: params.release_link_base_url,
        compare_link_base_url: params.compare_link_base_url,
        skip_merge_commits: params
            .changelog_config
            .skip_merge_commits
            .unwrap_or(DEFAULT_SKIP_MERGE_COMMITS),
        tag_prefix: Some(params.tag_prefix),
        commit_modifiers: params.commit_modifiers.clone(),
        named_parsers: params
            .changelog_config
            .named_parsers
            .unwrap_or_default(),
        custom_parsers: params.changelog_config.custom_parsers,
    }
}
