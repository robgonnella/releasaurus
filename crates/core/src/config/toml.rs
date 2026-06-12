//! Configuration loading and parsing for `releasaurus.toml` files.
//!
//! Supports customizable changelog templates and multi-package repositories.
use derive_builder::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    config::{
        changelog::ChangelogConfig, package::PackageConfig,
        prerelease::PrereleaseConfig,
    },
    result::{ReleasaurusError, Result},
};

/// Default configuration filename
pub const DEFAULT_CONFIG_FILE: &str = "releasaurus.toml";
/// Default number of commits to search when processing first release
pub const DEFAULT_COMMIT_SEARCH_DEPTH: usize = 400;
/// Default number of tags to search when looking for previous releases
pub const DEFAULT_TAG_SEARCH_DEPTH: usize = 100;

/// Determines what type of versioning to use (semantic, date, etc.)
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Default,
)]
pub enum VersionType {
    #[default]
    #[serde(rename = "major.minor.patch")]
    Semantic,
    #[serde(rename = "major.minor.patch+timestamp.sha")]
    SemanticWithBuild,
    #[serde(rename = "year.month.day")]
    Date,
    #[serde(rename = "year.month.day+hour.minute.second")]
    DateWithTime,
    #[serde(rename = "year.month.day+hour.minute.second.micro")]
    DateWithTimeMicro,
}

impl std::fmt::Display for VersionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            VersionType::Semantic => "major.minor.patch",
            VersionType::SemanticWithBuild => "major.minor.patch+timestamp.sha",
            VersionType::Date => "year.month.day",
            VersionType::DateWithTime => "year.month.day+hour.minute.second",
            VersionType::DateWithTimeMicro => {
                "year.month.day+hour.minute.second.micro"
            }
        };
        write!(f, "{s}")
    }
}

impl VersionType {
    /// Returns true for date-based version types, which derive the version
    /// from the current date/time rather than from commits. Semantic-only
    /// settings (prerelease, custom increment regexes) do not apply to
    /// these types.
    pub fn is_date_based(&self) -> bool {
        matches!(
            self,
            VersionType::Date
                | VersionType::DateWithTime
                | VersionType::DateWithTimeMicro
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Builder)]
#[schemars(rename = "Releasaurus TOML Configuration Schema")]
#[serde(default)]
#[builder(setter(into, strip_option), default)]
/// Configuration properties for `releasaurus.toml`
pub struct Config {
    /// The base branch to target for release PRs, tagging, and releases
    /// defaults to default_branch for repository
    pub base_branch: Option<String>,
    /// Maximum number of commits to search for the first release when no
    /// tags exist
    pub first_release_search_depth: usize,
    /// Maximum number of tags to pull when searching for previous releases.
    /// Set to 0 to search all tags
    pub tag_search_depth: usize,
    /// Generates different release PRs for each package defined in config
    pub separate_pull_requests: bool,
    /// Global prerelease configuration (suffix + strategy). Packages can
    /// override this configuration
    /// Only applies when version_type is major.minor.patch or
    /// major.minor.patch+timestamp.sha
    pub prerelease: PrereleaseConfig,
    /// Global config to auto start next release for all packages. Packages
    /// can override this configuration
    pub auto_start_next: Option<bool>,
    /// Determines what kind of versioning to perform (semantic, date, etc).
    /// Packages can override this configuration.
    /// Default: major.minor.patch (semantic)
    pub version_type: Option<VersionType>,
    /// Always increments major version on breaking commits.
    /// Packages can override this configuration. Default: true.
    /// Only applies when version_type is major.minor.patch or
    /// major.minor.patch+timestamp.sha
    pub breaking_always_increment_major: Option<bool>,
    /// Always increments minor version on feature commits.
    /// Packages can override this configuration. Default: true.
    /// Only applies when version_type is major.minor.patch or
    /// major.minor.patch+timestamp.sha
    pub features_always_increment_minor: Option<bool>,
    /// Custom regex pattern matched against commit messages to trigger a
    /// major version bump. This is additive — breaking change commits always
    /// trigger major bumps regardless of this setting. In TOML double-quoted
    /// strings, escape backslashes (e.g. `"\\[BREAKING\\]"` matches
    /// `[BREAKING]`). Only applies when version_type is major.minor.patch
    /// or major.minor.patch+timestamp.sha
    pub custom_major_increment_regex: Option<String>,
    /// Custom regex pattern matched against commit messages to trigger a
    /// minor version bump. This is additive — `feat:` commits always trigger
    /// minor bumps regardless of this setting. In TOML double-quoted strings,
    /// escape backslashes (e.g. `"\\[FEATURE\\]"` matches `[FEATURE]`).
    /// Only applies when version_type is major.minor.patch or
    /// major.minor.patch+timestamp.sha
    pub custom_minor_increment_regex: Option<String>,
    /// Changelog generation settings.
    pub changelog: ChangelogConfig,
    /// Packages to manage in this repository (supports monorepos)
    #[serde(rename = "package")]
    pub packages: Vec<PackageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_branch: None,
            first_release_search_depth: DEFAULT_COMMIT_SEARCH_DEPTH,
            tag_search_depth: DEFAULT_TAG_SEARCH_DEPTH,
            separate_pull_requests: false,
            prerelease: PrereleaseConfig::default(),
            auto_start_next: None,
            version_type: None,
            breaking_always_increment_major: None,
            features_always_increment_minor: None,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            changelog: ChangelogConfig::default(),
            packages: vec![PackageConfig::default()],
        }
    }
}

impl Config {
    pub fn base_branch(&self) -> Result<String> {
        self.base_branch
            .clone()
            .ok_or_else(|| ReleasaurusError::BaseBranchNotConfigured)
    }

    pub fn auto_start_next(&self, package: &PackageConfig) -> bool {
        package
            .auto_start_next
            .or(self.auto_start_next)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_type_is_date_based() {
        assert!(VersionType::Date.is_date_based());
        assert!(VersionType::DateWithTime.is_date_based());
        assert!(VersionType::DateWithTimeMicro.is_date_based());
        assert!(!VersionType::Semantic.is_date_based());
        assert!(!VersionType::SemanticWithBuild.is_date_based());
    }

    #[test]
    fn loads_defaults() {
        let config = Config::default();
        assert!(!config.changelog.body.is_empty());
        assert_eq!(
            config.first_release_search_depth,
            DEFAULT_COMMIT_SEARCH_DEPTH
        );
    }

    #[test]
    fn base_branch_returns_value_when_set() {
        let config = Config {
            base_branch: Some("main".into()),
            ..Default::default()
        };

        assert_eq!(config.base_branch().unwrap(), "main");
    }

    #[test]
    fn base_branch_returns_error_when_none() {
        let config = Config {
            base_branch: None,
            ..Default::default()
        };

        assert!(config.base_branch().is_err());
    }

    #[test]
    fn auto_start_next_uses_package_override() {
        let config = Config {
            auto_start_next: Some(false),
            ..Default::default()
        };
        let package = PackageConfig {
            auto_start_next: Some(true),
            ..Default::default()
        };

        assert!(config.auto_start_next(&package));
    }

    #[test]
    fn auto_start_next_uses_global_when_package_not_set() {
        let config = Config {
            auto_start_next: Some(true),
            ..Default::default()
        };
        let package = PackageConfig::default();

        assert!(config.auto_start_next(&package));
    }

    #[test]
    fn auto_start_next_defaults_to_false() {
        let config = Config::default();
        let package = PackageConfig::default();

        assert!(!config.auto_start_next(&package));
    }
}
