//! Analyzer configuration resolution.
//!
//! Builds AnalyzerConfig instances from resolved package parameters,
//! handling complex interactions between global config, package
//! config, and CLI overrides.

use regex::Regex;
use std::rc::Rc;

use crate::{
    OrchestratorConfig, analyzer::config::AnalyzerConfig,
    config::prerelease::PrereleaseConfig,
};

/// Parameters for building an analyzer configuration.
///
/// This is an internal type used to pass resolved values from
/// package configuration into the analyzer config builder.
#[derive(Debug)]
pub struct AnalyzerParams {
    pub config: Rc<OrchestratorConfig>,
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
    let release_commit_matcher = build_release_commit_matcher(
        &params.config.base_branch,
        &params.package_name,
    );

    AnalyzerConfig {
        body: params.config.changelog.body.clone(),
        breaking_always_increment_major: params.breaking_always_increment_major,
        custom_major_increment_regex: params.custom_major_increment_regex,
        custom_minor_increment_regex: params.custom_minor_increment_regex,
        features_always_increment_minor: params.features_always_increment_minor,
        include_author: params.config.changelog.include_author,
        prerelease: params.prerelease,
        release_commit_matcher,
        release_link_base_url: params.config.release_link_base_url.clone(),
        compare_link_base_url: params.config.compare_link_base_url.clone(),
        skip_chore: params.config.changelog.skip_chore,
        skip_ci: params.config.changelog.skip_ci,
        skip_merge_commits: params.config.changelog.skip_merge_commits,
        skip_miscellaneous: params.config.changelog.skip_miscellaneous,
        skip_release_commits: params.config.changelog.skip_release_commits,
        tag_prefix: Some(params.tag_prefix),
        commit_modifiers: params.config.commit_modifiers.clone(),
    }
}

/// Builds a regex matcher for release commits for this package.
///
/// Release commits follow the pattern:
/// `chore(base_branch): release package_name`
fn build_release_commit_matcher(
    base_branch: &str,
    package_name: &str,
) -> Option<Regex> {
    Regex::new(&format!(
        r#"^chore\({}\): release {}"#,
        regex::escape(base_branch),
        regex::escape(package_name)
    ))
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_release_commit_matcher_correctly() {
        let matcher = build_release_commit_matcher("main", "my-package");
        assert!(matcher.is_some());

        let regex = matcher.unwrap();
        assert!(regex.is_match("chore(main): release my-package"));
        assert!(!regex.is_match("chore(main): release other-package"));
        assert!(!regex.is_match("chore(dev): release my-package"));
    }

    #[test]
    fn escapes_special_regex_characters() {
        let matcher = build_release_commit_matcher("main", "my-package");
        assert!(matcher.is_some());

        let regex = matcher.unwrap();
        // Parentheses in package name should be escaped
        assert!(!regex.is_match("chore(main): release my(package"));
    }
}
