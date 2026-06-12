//! Analyzer configuration resolution.
//!
//! Builds AnalyzerConfig instances from resolved package parameters,
//! handling complex interactions between global config, package
//! config, and CLI overrides.

use url::Url;

use crate::{
    analyzer::config::AnalyzerConfig,
    config::{
        changelog::{ChangelogConfig, DEFAULT_BODY, DEFAULT_INCLUDE_AUTHOR},
        overrides::CommitModifiers,
        versioning::{
            DEFAULT_SKIP_MERGE_COMMITS, DEFAULT_VERSION_TYPE, NAMED_PARSERS,
            VersioningConfig,
        },
    },
};

/// Parameters for building an analyzer configuration.
///
/// This is an internal type used to pass resolved values from
/// package configuration into the analyzer config builder.
#[derive(Debug)]
pub struct AnalyzerParams {
    pub changelog: ChangelogConfig,
    pub versioning: VersioningConfig,
    pub tag_prefix: String,
    pub release_link_base_url: Option<Url>,
    pub compare_link_base_url: Option<Url>,
    pub commit_modifiers: CommitModifiers,
}

/// Builds an AnalyzerConfig from resolved parameters.
///
/// This function combines global configuration, package-specific
/// settings, and generates package-specific patterns (like release
/// commit matcher).
pub fn build_analyzer_config(params: AnalyzerParams) -> AnalyzerConfig {
    AnalyzerConfig {
        version_type: params
            .versioning
            .version_type
            .unwrap_or(DEFAULT_VERSION_TYPE),
        body: params.changelog.body.unwrap_or_else(|| DEFAULT_BODY.into()),
        breaking_always_increment_major: params
            .versioning
            .breaking_always_increment_major,
        custom_major_increment_regex: params
            .versioning
            .custom_major_increment_regex,
        custom_minor_increment_regex: params
            .versioning
            .custom_minor_increment_regex,
        features_always_increment_minor: params
            .versioning
            .features_always_increment_minor,
        include_author: params
            .changelog
            .include_author
            .unwrap_or(DEFAULT_INCLUDE_AUTHOR),
        prerelease: params.versioning.prerelease,
        release_link_base_url: params.release_link_base_url,
        compare_link_base_url: params.compare_link_base_url,
        skip_merge_commits: params
            .versioning
            .skip_merge_commits
            .unwrap_or(DEFAULT_SKIP_MERGE_COMMITS),
        tag_prefix: Some(params.tag_prefix),
        commit_modifiers: params.commit_modifiers,
        // `resolve_versioning` always populates this, but an empty map would
        // classify every commit into an unnamed group and apply no `skip`, so
        // fall back to the built-in defaults rather than `IndexMap::default()`.
        named_parsers: params
            .versioning
            .named_parsers
            .unwrap_or_else(|| NAMED_PARSERS.clone()),
        custom_parsers: params.versioning.custom_parsers.unwrap_or_default().0,
    }
}
