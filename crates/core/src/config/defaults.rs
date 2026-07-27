use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::{
    changelog::ChangelogConfig,
    versioning::{
        DEFAULT_BREAKING_ALWAYS_INCREMENT_MAJOR,
        DEFAULT_FEAT_ALWAYS_INCREMENT_MINOR, DEFAULT_SKIP_MERGE_COMMITS,
        DEFAULT_VERSION_TYPE, NAMED_PARSERS, VersioningConfig,
    },
};

pub const DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE: &str =
    "chore({{ branch }}): release {{ package_name }} {{ tag }}";

pub const DEFAULT_MONOREPO_COMMIT_AND_PR_TITLE_TEMPLATE: &str =
    "chore({{ branch }}): release {{ repo_name }}";

fn default_commit_and_pr_title() -> String {
    DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE.into()
}

fn default_monorepo_commit_and_pr_title() -> String {
    DEFAULT_MONOREPO_COMMIT_AND_PR_TITLE_TEMPLATE.into()
}

fn default_versioning() -> VersioningConfig {
    VersioningConfig {
        version_type: Some(DEFAULT_VERSION_TYPE),
        prerelease: None,
        auto_start_next: Some(false),
        breaking_always_increment_major: Some(
            DEFAULT_BREAKING_ALWAYS_INCREMENT_MAJOR,
        ),
        features_always_increment_minor: Some(
            DEFAULT_FEAT_ALWAYS_INCREMENT_MINOR,
        ),
        custom_major_increment_regex: None,
        custom_minor_increment_regex: None,
        skip_merge_commits: Some(DEFAULT_SKIP_MERGE_COMMITS),
        named_parsers: Some(NAMED_PARSERS.clone()),
        custom_parsers: None,
    }
}

/// Default configuration applied to every package
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)] // Use default for missing fields
pub struct DefaultsConfig {
    /// Tera template for generating release commit messages when
    /// repository.separate_pull_requests=false and multiple packages
    /// configured. Has the following variables available in the template
    /// context: branch, repo_name
    #[schemars(default = "default_monorepo_commit_and_pr_title")]
    pub monorepo_commit_message_template: Option<String>,
    /// Tera template for generating release PR titles when
    /// repository.separate_pull_requests=false and multiple packages
    /// configured. Has the following variables available in the template
    /// context: branch, repo_name
    #[schemars(default = "default_monorepo_commit_and_pr_title")]
    pub monorepo_pr_title_template: Option<String>,
    /// Tera template for generating release commit messages. When
    /// repository.separate_pull_requests=true, or only one package configured,
    /// this template will be used for each individual PR commit but can be
    /// overridden at the package level. Has the following variables available
    /// in the template context: branch, repo_name, package_name, tag, semver
    #[schemars(default = "default_commit_and_pr_title")]
    pub commit_message_template: Option<String>,
    /// Tera template for generating release PR titles. When
    /// repository.separate_pull_requests=true, or only one package configured,
    /// this template will be used for each individual PR title but can be
    /// overridden at the package level. Has the following variables available
    /// in the template context: branch, repo_name, package_name, tag, semver
    #[schemars(default = "default_commit_and_pr_title")]
    pub pr_title_template: Option<String>,
    /// Default versioning config. Packages can override this configuration
    #[schemars(default = "default_versioning")]
    pub versioning: Option<VersioningConfig>,
    /// Default changelog generation settings applied to all packages.
    /// Packages can override this configuration
    pub changelog: Option<ChangelogConfig>,
}
