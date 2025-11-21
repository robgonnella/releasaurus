//! Common functionality shared between release commands
use std::path::Path;

use crate::{
    analyzer::config::AnalyzerConfig,
    config::{Config, PackageConfig},
    forge::{config::RemoteConfig, request::ForgeCommit, traits::Forge},
    result::Result,
};

pub fn process_config(repo_name: &str, config: &mut Config) -> Config {
    for package in config.packages.iter_mut() {
        package.name = derive_package_name(package, repo_name);
    }
    // drop mutability
    config.clone()
}

/// Resolve tag prefix for package from config or generate default based on
/// package path for monorepo support.
pub fn get_tag_prefix(package: &PackageConfig, repo_name: &str) -> String {
    let mut default_for_package = "v".to_string();
    let name = derive_package_name(package, repo_name);
    if package.path != "." {
        default_for_package = format!("{}-v", name);
    }
    package.tag_prefix.clone().unwrap_or(default_for_package)
}

/// Determines prerelease identifier with consistent priority logic.
///
/// This function is used by both `release-pr` and `release` commands to ensure
/// consistent prerelease version behavior across the entire workflow.
///
/// # Priority
/// CLI override > package config > global config
pub fn get_prerelease(
    config: &Config,
    package: &PackageConfig,
    cli_override: Option<String>,
) -> Option<String> {
    cli_override
        .or_else(|| package.prerelease.clone())
        .or_else(|| config.prerelease.clone())
}

/// Generates [`AnalyzerConfig`] from [`Config`], [`RemoteConfig`],
/// [`PackageConfig`], and tag_prefix [`String`].
/// Prerelease priority: CLI override > package config > global config
pub fn generate_analyzer_config(
    config: &Config,
    remote_config: &RemoteConfig,
    package: &PackageConfig,
    tag_prefix: String,
    prerelease_override: Option<String>,
) -> AnalyzerConfig {
    // Determine prerelease with priority: override > package > global
    let prerelease = get_prerelease(config, package, prerelease_override);

    AnalyzerConfig {
        body: config.changelog.body.clone(),
        include_author: config.changelog.include_author,
        skip_chore: config.changelog.skip_chore,
        skip_ci: config.changelog.skip_ci,
        skip_miscellaneous: config.changelog.skip_miscellaneous,
        release_link_base_url: remote_config.release_link_base_url.clone(),
        tag_prefix: Some(tag_prefix),
        prerelease,
    }
}

/// Extract package name from its path, using repository name for root
/// packages.
pub fn derive_package_name(package: &PackageConfig, repo_name: &str) -> String {
    if !package.name.is_empty() {
        return package.name.to_string();
    }

    let path = Path::new(&package.path);

    if let Some(name) = path.file_name() {
        return name.display().to_string();
    }

    if package.path == "." {
        // For root package, use repository directory name as fallback
        repo_name.into()
    } else {
        // Extract name from path (e.g., "crates/my-package" -> "my-package")
        package
            .path
            .split('/')
            .next_back()
            .unwrap_or(&package.path)
            .to_string()
    }
}

/// Retrieves commits for specific package from forge
pub async fn get_package_commits(
    forge: &dyn Forge,
    starting_sha: Option<String>,
    package_paths: &[String],
) -> Result<Vec<ForgeCommit>> {
    let commits = forge.get_commits(starting_sha).await?;

    let mut package_commits: Vec<ForgeCommit> = vec![];

    for commit in commits.iter() {
        for file in commit.files.iter() {
            let file_path = Path::new(file);
            for package_path in package_paths.iter() {
                let normalized_path = package_path.replace("./", "");
                let mut normalized_path = Path::new(&normalized_path);
                if package_path == "." {
                    normalized_path = Path::new("");
                }
                if file_path.starts_with(normalized_path) {
                    package_commits.push(commit.clone());
                }
            }
        }
    }

    Ok(package_commits)
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{PackageConfig, ReleaseType},
        test_helpers,
    };

    use super::*;

    #[test]
    fn test_get_tag_prefix_using_standard_default() {
        let repo_name = "test-repo";

        let package = PackageConfig {
            name: "my-package".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: None,
        };

        let tag_prefix = get_tag_prefix(&package, repo_name);

        assert_eq!(tag_prefix, "v");
    }

    #[test]
    fn test_get_tag_prefix_using_name() {
        let repo_name = "test-repo";

        let package = PackageConfig {
            name: "my-package".into(),
            path: "packages/my-package".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: None,
        };

        let tag_prefix = get_tag_prefix(&package, repo_name);

        assert_eq!(tag_prefix, "my-package-v");
    }

    #[test]
    fn test_get_tag_prefix_using_path() {
        let repo_name = "test-repo";

        let package = PackageConfig {
            name: "".into(),
            path: "packages/my-package".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: None,
        };

        let tag_prefix = get_tag_prefix(&package, repo_name);

        assert_eq!(tag_prefix, "my-package-v");
    }

    #[test]
    fn test_get_tag_prefix_using_configured_prefix() {
        let repo_name = "test-repo";

        let package = PackageConfig {
            name: "my-package".into(),
            path: "packages/my-package".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: Some("my-special-tag-prefix-v".into()),
            prerelease: None,
            additional_paths: None,
        };

        let tag_prefix = get_tag_prefix(&package, repo_name);

        assert_eq!(tag_prefix, "my-special-tag-prefix-v");
    }

    #[test]
    fn test_derive_package_name_from_directory() {
        let mut package = PackageConfig {
            name: "".into(),
            path: "packages/my-package".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: Some("v".into()),
            prerelease: None,
            additional_paths: None,
        };
        // Test with simple directory name
        let name = derive_package_name(&package, "test-repo");
        assert_eq!(name, "my-package");

        // Test with nested path
        package.name = "".into();
        package.path = "crates/core/utils".into();
        let name = derive_package_name(&package, "test-repo");
        assert_eq!(name, "utils");

        // Test with root path
        package.name = "".into();
        package.path = ".".into();
        let name = derive_package_name(&package, "test-repo");
        assert_eq!(name, "test-repo");

        // Test with single directory
        package.name = "".into();
        package.path = "backend".into();
        let name = derive_package_name(&package, "test-repo");
        assert_eq!(name, "backend");
    }

    #[test]
    fn test_process_config_derives_package_names() {
        let repo_name = "test-repo";

        let mut config = test_helpers::create_test_config(vec![
            PackageConfig {
                name: "".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Generic),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            },
            PackageConfig {
                name: "".into(),
                path: "packages/api".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            },
        ]);

        let processed = process_config(repo_name, &mut config);

        assert_eq!(processed.packages[0].name, "test-repo");
        assert_eq!(processed.packages[1].name, "api");
    }

    #[test]
    fn test_process_config_preserves_existing_names() {
        let repo_name = "test-repo";

        let mut config = test_helpers::create_test_config(vec![
            PackageConfig {
                name: "my-custom-name".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Generic),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            },
            PackageConfig {
                name: "another-name".into(),
                path: "packages/api".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            },
        ]);

        let processed = process_config(repo_name, &mut config);

        assert_eq!(processed.packages[0].name, "my-custom-name");
        assert_eq!(processed.packages[1].name, "another-name");
    }

    #[test]
    fn test_process_config_mixed_names() {
        let repo_name = "test-repo";

        let mut config = test_helpers::create_test_config(vec![
            PackageConfig {
                name: "explicit-name".into(),
                path: "packages/frontend".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Generic),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            },
            PackageConfig {
                name: "".into(),
                path: "packages/backend".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            },
        ]);

        let processed = process_config(repo_name, &mut config);

        assert_eq!(processed.packages[0].name, "explicit-name");
        assert_eq!(processed.packages[1].name, "backend");
    }

    #[test]
    fn test_derive_package_name_with_explicit_name() {
        let package = PackageConfig {
            name: "explicit-package-name".into(),
            path: "packages/something".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: None,
        };

        let name = derive_package_name(&package, "test-repo");
        assert_eq!(name, "explicit-package-name");
    }

    #[test]
    fn test_get_prerelease_cli_override_takes_priority() {
        let mut config =
            test_helpers::create_test_config(vec![PackageConfig {
                name: "my-package".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Generic),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            }]);
        // Set all three levels
        config.prerelease = Some("alpha".to_string());
        config.packages[0].prerelease = Some("beta".to_string());
        let cli_override = Some("rc".to_string());

        let result = get_prerelease(&config, &config.packages[0], cli_override);

        // CLI override should win
        assert_eq!(result, Some("rc".to_string()));
    }

    #[test]
    fn test_get_prerelease_package_overrides_global() {
        let mut config =
            test_helpers::create_test_config(vec![PackageConfig {
                name: "my-package".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Generic),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            }]);
        // Set both global and package
        config.prerelease = Some("alpha".to_string());
        config.packages[0].prerelease = Some("beta".to_string());

        let result = get_prerelease(&config, &config.packages[0], None);

        // Package should win over global
        assert_eq!(result, Some("beta".to_string()));
    }

    #[test]
    fn test_get_prerelease_uses_global_when_package_not_set() {
        let mut config =
            test_helpers::create_test_config(vec![PackageConfig {
                name: "my-package".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Generic),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            }]);
        // Set only global
        config.prerelease = Some("alpha".to_string());

        let result = get_prerelease(&config, &config.packages[0], None);

        // Should use global
        assert_eq!(result, Some("alpha".to_string()));
    }

    #[test]
    fn test_get_prerelease_returns_none_when_nothing_set() {
        let config = test_helpers::create_test_config(vec![PackageConfig {
            name: "my-package".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: None,
        }]);
        // Nothing set

        let result = get_prerelease(&config, &config.packages[0], None);

        // Should return None
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_prerelease_cli_override_works_alone() {
        let config = test_helpers::create_test_config(vec![PackageConfig {
            name: "my-package".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: None,
        }]);
        // Only CLI override set
        let cli_override = Some("dev".to_string());

        let result = get_prerelease(&config, &config.packages[0], cli_override);

        // Should use CLI override
        assert_eq!(result, Some("dev".to_string()));
    }

    #[test]
    fn test_get_prerelease_consistency_between_commands() {
        // This test verifies that both release-pr and release commands
        // would get the same prerelease value given the same inputs
        let mut config =
            test_helpers::create_test_config(vec![PackageConfig {
                name: "my-package".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Generic),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            }]);

        // Scenario 1: Package config overrides global
        config.prerelease = Some("alpha".to_string());
        config.packages[0].prerelease = Some("beta".to_string());

        let result_pr = get_prerelease(&config, &config.packages[0], None);
        let result_release = get_prerelease(&config, &config.packages[0], None);

        assert_eq!(result_pr, result_release);
        assert_eq!(result_pr, Some("beta".to_string()));

        // Scenario 2: CLI override takes priority
        let cli_override = Some("rc".to_string());

        let result_pr_cli =
            get_prerelease(&config, &config.packages[0], cli_override.clone());
        let result_release_cli =
            get_prerelease(&config, &config.packages[0], cli_override);

        assert_eq!(result_pr_cli, result_release_cli);
        assert_eq!(result_pr_cli, Some("rc".to_string()));
    }
}
