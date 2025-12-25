//! Configuration loading and parsing for `releasaurus.toml` files.
//!
//! Supports customizable changelog templates and multi-package repositories.
use std::{collections::HashMap, path::Path};

use color_eyre::eyre::eyre;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    Result,
    analyzer::config::AnalyzerConfig,
    cli::{GlobalOverrides, PackageOverrides},
    config::package::{DEFAULT_TAG_PREFIX, PackageConfig},
    forge::config::DEFAULT_COMMIT_SEARCH_DEPTH,
};

pub mod changelog;
pub mod package;
pub mod prerelease;
pub mod release_type;

use self::prerelease::PrereleaseConfig;

/// Default configuration filename
pub const DEFAULT_CONFIG_FILE: &str = "releasaurus.toml";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Releasaurus TOML Configuration Schema")]
#[serde(default)]
/// Configuration properties for `releasaurus.toml`
pub struct Config {
    /// The base branch to target for release PRs, tagging, and releases
    /// defaults to default_branch for repository
    pub base_branch: Option<String>,
    /// Maximum number of commits to search for the first release when no
    /// tags exist
    pub first_release_search_depth: u64,
    /// Generates different release PRs for each package defined in config
    pub separate_pull_requests: bool,
    /// Global prerelease configuration (suffix + strategy). Packages can
    /// override this configuration
    pub prerelease: PrereleaseConfig,
    /// Global config to auto start next release for all packages. Packages
    /// can override this configuration
    pub auto_start_next: Option<bool>,
    /// Always increments major version on breaking commits
    pub breaking_always_increment_major: bool,
    /// Always increments minor version on feature commits
    pub features_always_increment_minor: bool,
    /// Custom commit type regex matcher to increment major version
    pub custom_major_increment_regex: Option<String>,
    /// Custom commit type regex matcher to increment minor version
    pub custom_minor_increment_regex: Option<String>,
    /// Changelog generation settings.
    pub changelog: changelog::ChangelogConfig,
    /// Packages to manage in this repository (supports monorepos)
    #[serde(rename = "package")]
    pub packages: Vec<PackageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_branch: None,
            first_release_search_depth: DEFAULT_COMMIT_SEARCH_DEPTH,
            separate_pull_requests: false,
            prerelease: PrereleaseConfig::default(),
            auto_start_next: None,
            breaking_always_increment_major: true,
            features_always_increment_minor: true,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            changelog: changelog::ChangelogConfig::default(),
            packages: vec![package::PackageConfig::default()],
        }
    }
}

impl Config {
    /// Preforms resolution of all derived properties
    pub fn resolve(
        &mut self,
        repo_name: &str,
        repo_default_branch: &str,
        release_link_base_url: &str,
        package_overrides: HashMap<String, PackageOverrides>,
        global_overrides: GlobalOverrides,
    ) -> Config {
        let base_branch = global_overrides
            .base_branch
            .clone()
            .or(self.base_branch.clone())
            .unwrap_or(repo_default_branch.to_string());

        self.base_branch = Some(base_branch.clone());

        for package in self.packages.iter_mut() {
            let package_name = if !package.name.is_empty() {
                package.name.to_string()
            } else if let Some(name) = Path::new(&package.workspace_root)
                .join(&package.path)
                .file_name()
            {
                name.display().to_string()
            } else {
                repo_name.to_string()
            };

            package.name = package_name;

            let mut default_tag_prefix = DEFAULT_TAG_PREFIX.to_string();
            if package.workspace_root != "." || package.path != "." {
                default_tag_prefix = format!("{}-v", package.name);
            }

            package.tag_prefix =
                package.tag_prefix.clone().or(Some(default_tag_prefix));

            package.auto_start_next =
                package.auto_start_next.or(self.auto_start_next);

            // start at lowest level and override each property according to next
            // level of precedence
            let mut prerelease = self.prerelease.clone();

            // package config overrides global config
            if let Some(pkg_prerelease) = package.prerelease.clone() {
                prerelease = pkg_prerelease;
            }

            // global cli overrides any config defined in file
            if global_overrides.prerelease_suffix.is_some() {
                prerelease.suffix = global_overrides.prerelease_suffix.clone();
            }

            if let Some(strategy) = global_overrides.prerelease_strategy {
                prerelease.strategy = strategy;
            }

            // package specific cli overrides take precedence over all
            if let Some(overrides) = package_overrides.get(&package.name) {
                if overrides.prerelease_suffix.is_some() {
                    prerelease.suffix = overrides.prerelease_suffix.clone();
                }

                if overrides.prerelease_strategy.is_some() {
                    prerelease.strategy =
                        overrides.prerelease_strategy.unwrap_or_default();
                }
            }

            // convert empty ("") suffix to None
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

            let mut release_commit_matcher = None;

            if let Ok(matcher) = Regex::new(&format!(
                r#"^chore\({base_branch}\): release {}"#,
                package.name
            )) {
                release_commit_matcher = Some(matcher);
            }

            let breaking_always_increment_major = package
                .breaking_always_increment_major
                .unwrap_or(self.breaking_always_increment_major);

            let features_always_increment_minor = package
                .features_always_increment_minor
                .unwrap_or(self.features_always_increment_minor);

            let custom_major_increment_regex = package
                .custom_major_increment_regex
                .clone()
                .or(self.custom_major_increment_regex.clone());

            let custom_minor_increment_regex = package
                .custom_minor_increment_regex
                .clone()
                .or(self.custom_minor_increment_regex.clone());

            package.analyzer_config = AnalyzerConfig {
                body: self.changelog.body.clone(),
                breaking_always_increment_major,
                custom_major_increment_regex,
                custom_minor_increment_regex,
                features_always_increment_minor,
                include_author: self.changelog.include_author,
                prerelease: package.prerelease.clone(),
                release_commit_matcher,
                release_link_base_url: release_link_base_url.to_string(),
                skip_chore: self.changelog.skip_chore,
                skip_ci: self.changelog.skip_ci,
                skip_merge_commits: self.changelog.skip_merge_commits,
                skip_miscellaneous: self.changelog.skip_miscellaneous,
                skip_release_commits: self.changelog.skip_release_commits,
                tag_prefix: package.tag_prefix.clone(),
            }
        }

        // drop mutability
        self.clone()
    }

    pub fn base_branch(&self) -> Result<String> {
        self.base_branch
            .clone()
            .ok_or(eyre!("failed to resolve base_branch"))
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
    use crate::{
        config::prerelease::PrereleaseStrategy,
        forge::config::DEFAULT_COMMIT_SEARCH_DEPTH,
    };

    use super::*;

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

    #[test]
    fn resolve_sets_base_branch_from_global_override() {
        let mut config = Config::default();
        let global_overrides = GlobalOverrides {
            base_branch: Some("develop".into()),
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            global_overrides,
        );

        assert_eq!(resolved.base_branch, Some("develop".into()));
    }

    #[test]
    fn resolve_sets_base_branch_from_config() {
        let mut config = Config {
            base_branch: Some("staging".into()),
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        assert_eq!(resolved.base_branch, Some("staging".into()));
    }

    #[test]
    fn resolve_uses_repo_default_branch_as_fallback() {
        let mut config = Config::default();

        let resolved = config.resolve(
            "test-repo",
            "trunk",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        assert_eq!(resolved.base_branch, Some("trunk".into()));
    }

    #[test]
    fn resolve_derives_package_name_from_repo() {
        let mut config = Config {
            packages: vec![PackageConfig {
                path: ".".into(),
                workspace_root: ".".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = config.resolve(
            "my-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        assert_eq!(resolved.packages[0].name, "my-repo");
    }

    #[test]
    fn resolve_derives_package_name_from_path() {
        let mut config = Config {
            packages: vec![PackageConfig {
                path: "packages/api".into(),
                workspace_root: ".".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        assert_eq!(resolved.packages[0].name, "api");
    }

    #[test]
    fn resolve_preserves_explicit_package_name() {
        let mut config = Config {
            packages: vec![PackageConfig {
                name: "custom-name".into(),
                path: "packages/api".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        assert_eq!(resolved.packages[0].name, "custom-name");
    }

    #[test]
    fn resolve_sets_default_tag_prefix_for_root() {
        let mut config = Config {
            packages: vec![PackageConfig {
                path: ".".into(),
                workspace_root: ".".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        assert_eq!(resolved.packages[0].tag_prefix, Some("v".into()));
    }

    #[test]
    fn resolve_sets_package_name_tag_prefix_for_subdir() {
        let mut config = Config {
            packages: vec![PackageConfig {
                path: "packages/api".into(),
                workspace_root: ".".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        assert_eq!(resolved.packages[0].tag_prefix, Some("api-v".into()));
    }

    #[test]
    fn resolve_sets_configured_package_tag_prefix() {
        let mut config = Config {
            packages: vec![PackageConfig {
                path: "packages/api".into(),
                workspace_root: ".".into(),
                tag_prefix: Some("my-prefix-v".into()),
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        assert_eq!(resolved.packages[0].tag_prefix, Some("my-prefix-v".into()));
    }

    #[test]
    fn resolve_prerelease_package_overrides_global() {
        let mut config = Config {
            prerelease: PrereleaseConfig {
                suffix: Some("alpha".into()),
                strategy: PrereleaseStrategy::Static,
            },
            packages: vec![PackageConfig {
                name: "pkg".into(),
                prerelease: Some(PrereleaseConfig {
                    suffix: Some("beta".into()),
                    strategy: PrereleaseStrategy::Versioned,
                }),
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        let prerelease = resolved.packages[0].prerelease.as_ref().unwrap();
        assert_eq!(prerelease.suffix, Some("beta".into()));
        assert_eq!(prerelease.strategy, PrereleaseStrategy::Versioned);
    }

    #[test]
    fn resolve_prerelease_global_cli_overrides_config() {
        let mut config = Config {
            prerelease: PrereleaseConfig {
                suffix: Some("alpha".into()),
                strategy: PrereleaseStrategy::Static,
            },
            packages: vec![PackageConfig {
                name: "pkg".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let global_overrides = GlobalOverrides {
            prerelease_suffix: Some("rc".into()),
            prerelease_strategy: Some(PrereleaseStrategy::Versioned),
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            global_overrides,
        );

        let prerelease = resolved.packages[0].prerelease.as_ref().unwrap();
        assert_eq!(prerelease.suffix, Some("rc".into()));
        assert_eq!(prerelease.strategy, PrereleaseStrategy::Versioned);
    }

    #[test]
    fn resolve_prerelease_package_cli_overrides_all() {
        let mut config = Config {
            prerelease: PrereleaseConfig {
                suffix: Some("alpha".into()),
                ..Default::default()
            },
            packages: vec![PackageConfig {
                name: "pkg".into(),
                prerelease: Some(PrereleaseConfig {
                    suffix: Some("beta".into()),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        };

        let global_overrides = GlobalOverrides {
            prerelease_suffix: Some("rc".into()),
            ..Default::default()
        };

        let mut package_overrides = HashMap::new();
        package_overrides.insert(
            "pkg".into(),
            PackageOverrides {
                prerelease_suffix: Some("gamma".into()),
                prerelease_strategy: Some(PrereleaseStrategy::Static),
            },
        );

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            package_overrides,
            global_overrides,
        );

        let prerelease = resolved.packages[0].prerelease.as_ref().unwrap();
        assert_eq!(prerelease.suffix, Some("gamma".into()));
        assert_eq!(prerelease.strategy, PrereleaseStrategy::Static);
    }

    #[test]
    fn resolve_removes_prerelease_when_suffix_empty() {
        let mut config = Config {
            packages: vec![PackageConfig {
                name: "pkg".into(),
                prerelease: Some(PrereleaseConfig {
                    suffix: Some("".into()),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        assert!(resolved.packages[0].prerelease.is_none());
    }

    #[test]
    fn resolve_trims_prerelease_suffix() {
        let mut config = Config {
            packages: vec![PackageConfig {
                name: "pkg".into(),
                prerelease: Some(PrereleaseConfig {
                    suffix: Some("  beta  ".into()),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        let prerelease = resolved.packages[0].prerelease.as_ref().unwrap();
        assert_eq!(prerelease.suffix, Some("beta".into()));
    }

    #[test]
    fn resolve_sets_analyzer_config_with_custom_regex() {
        let mut config = Config {
            custom_major_increment_regex: Some("BREAKING".into()),
            custom_minor_increment_regex: Some("FEATURE".into()),
            packages: vec![PackageConfig {
                name: "pkg".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        let analyzer_config = &resolved.packages[0].analyzer_config;
        assert_eq!(
            analyzer_config.custom_major_increment_regex,
            Some("BREAKING".into())
        );
        assert_eq!(
            analyzer_config.custom_minor_increment_regex,
            Some("FEATURE".into())
        );
    }

    #[test]
    fn resolve_package_custom_regex_overrides_global() {
        let mut config = Config {
            custom_major_increment_regex: Some("GLOBAL".into()),
            packages: vec![PackageConfig {
                name: "pkg".into(),
                custom_major_increment_regex: Some("PACKAGE".into()),
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        let analyzer_config = &resolved.packages[0].analyzer_config;
        assert_eq!(
            analyzer_config.custom_major_increment_regex,
            Some("PACKAGE".into())
        );
    }

    #[test]
    fn resolve_sets_analyzer_config_flags() {
        let mut config = Config {
            breaking_always_increment_major: false,
            features_always_increment_minor: false,
            packages: vec![PackageConfig {
                name: "pkg".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = config.resolve(
            "test-repo",
            "main",
            "https://example.com",
            HashMap::new(),
            GlobalOverrides::default(),
        );

        let analyzer_config = &resolved.packages[0].analyzer_config;
        assert!(!analyzer_config.breaking_always_increment_major);
        assert!(!analyzer_config.features_always_increment_minor);
    }
}
