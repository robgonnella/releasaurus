//! Common functionality shared between release commands
use log::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};

use crate::{
    Result,
    analyzer::{Analyzer, config::AnalyzerConfig, release::Tag},
    command::types::ReleasablePackage,
    config::{Config, package::PackageConfig, release_type::ReleaseType},
    forge::{
        config::RemoteConfig, manager::ForgeManager, request::ForgeCommit,
    },
    path_helpers::package_path,
    updater::manager::UpdateManager,
};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PRMetadataFields {
    pub name: String,
    pub tag: String,
    pub notes: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PRMetadata {
    pub metadata: PRMetadataFields,
}

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
    if package.workspace_root != "." || package.path != "." {
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
/// package config > global config
pub fn get_prerelease(
    config: &Config,
    package: &PackageConfig,
) -> Option<String> {
    package
        .prerelease
        .clone()
        .or_else(|| config.prerelease.clone())
}

/// Returns the prerelease_version flag with consistent priority logic.
///
/// This function is used by both `release-pr` and `release` commands to ensure
/// consistent prerelease version behavior across the entire workflow.
///
/// # Priority
/// package config > global config
pub fn get_prerelease_version(
    config: &Config,
    package: &PackageConfig,
) -> bool {
    package
        .prerelease_version
        .unwrap_or(config.prerelease_version)
}

/// Generates [`AnalyzerConfig`] from [`Config`], [`RemoteConfig`],
/// [`PackageConfig`], and tag_prefix [`String`].
/// Prerelease priority: CLI override > package config > global config
pub fn generate_analyzer_config(
    config: &Config,
    remote_config: &RemoteConfig,
    default_branch: &str,
    package: &PackageConfig,
    tag_prefix: String,
) -> AnalyzerConfig {
    // Determine prerelease with priority: override > package > global
    let prerelease = get_prerelease(config, package);
    let prerelease_version = get_prerelease_version(config, package);

    let mut release_commit_matcher = None;

    if let Ok(matcher) = Regex::new(&format!(
        r#"^chore\({default_branch}\): release {}"#,
        package.name
    )) {
        release_commit_matcher = Some(matcher);
    }

    let breaking_always_increment_major = package
        .breaking_always_increment_major
        .unwrap_or(config.breaking_always_increment_major);

    let features_always_increment_minor = package
        .features_always_increment_minor
        .unwrap_or(config.features_always_increment_minor);

    let custom_major_increment_regex = package
        .custom_major_increment_regex
        .clone()
        .or(config.custom_major_increment_regex.clone());

    let custom_minor_increment_regex = package
        .custom_minor_increment_regex
        .clone()
        .or(config.custom_minor_increment_regex.clone());

    AnalyzerConfig {
        body: config.changelog.body.clone(),
        include_author: config.changelog.include_author,
        skip_chore: config.changelog.skip_chore,
        skip_ci: config.changelog.skip_ci,
        skip_miscellaneous: config.changelog.skip_miscellaneous,
        skip_merge_commits: config.changelog.skip_merge_commits,
        skip_release_commits: config.changelog.skip_release_commits,
        release_link_base_url: remote_config.release_link_base_url.clone(),
        tag_prefix: Some(tag_prefix),
        prerelease,
        prerelease_version,
        release_commit_matcher,
        breaking_always_increment_major,
        features_always_increment_minor,
        custom_major_increment_regex,
        custom_minor_increment_regex,
    }
}

pub async fn get_releasable_packages(
    config: &Config,
    forge_manager: &ForgeManager,
) -> Result<Vec<ReleasablePackage>> {
    let default_branch = forge_manager.default_branch();
    let repo_name = forge_manager.repo_name();
    let remote_config = forge_manager.remote_config();

    let mut releasable_packages: Vec<ReleasablePackage> = vec![];

    let commits = get_commits_for_all_packages(
        forge_manager,
        &config.packages,
        &repo_name,
    )
    .await?;

    for package in config.packages.iter() {
        let tag_prefix = get_tag_prefix(package, &repo_name);
        let current_tag =
            forge_manager.get_latest_tag_for_prefix(&tag_prefix).await?;

        info!(
            "processing package: \n\tname: {}, \n\tworkspace_root: {}, \n\tpath: {}, \n\ttag_prefix: {}",
            package.name, package.workspace_root, package.path, tag_prefix
        );

        info!(
            "package_name: {}, current tag {:#?}",
            package.name, current_tag
        );

        let package_commits =
            filter_commits_for_package(package, current_tag.clone(), &commits);

        info!("processing commits for package: {}", package.name);

        let analyzer_config = generate_analyzer_config(
            config,
            &remote_config,
            &default_branch,
            package,
            tag_prefix.clone(),
        );

        let analyzer = Analyzer::new(analyzer_config)?;

        if let Some(release) = analyzer.analyze(package_commits, current_tag)? {
            info!("package: {}, release: {:#?}", package.name, release);

            let release_type =
                package.release_type.clone().unwrap_or(ReleaseType::Generic);

            let release_manifest_targets =
                UpdateManager::release_type_manifest_targets(package);

            let additional_manifest_targets =
                UpdateManager::additional_manifest_targets(package);

            let manifest_files = forge_manager
                .load_manifest_targets(release_manifest_targets)
                .await?;

            let additional_manifest_files = forge_manager
                .load_manifest_targets(additional_manifest_targets)
                .await?;

            releasable_packages.push(ReleasablePackage {
                name: package.name.clone(),
                path: package.path.clone(),
                workspace_root: package.workspace_root.clone(),
                manifest_files,
                additional_manifest_files,
                release_type,
                release,
            });
        } else {
            info!("nothing to release for package: {}", package.name);
        }
    }

    Ok(releasable_packages)
}

/// Extract package name from its path, using repository name for root
/// packages.
pub fn derive_package_name(package: &PackageConfig, repo_name: &str) -> String {
    if !package.name.is_empty() {
        return package.name.to_string();
    }

    let path = Path::new(&package.workspace_root).join(&package.path);

    if let Some(name) = path.file_name() {
        return name.display().to_string();
    }

    // if all else fails just return the name of the repository
    repo_name.into()
}

/// Retrieves all commits for all packages using the oldest found tag across
/// all packages. We do this once so we don't keep fetching the same commit
/// redundantly for each package.
pub async fn get_commits_for_all_packages(
    forge_manager: &ForgeManager,
    packages: &[PackageConfig],
    repo_name: &str,
) -> Result<Vec<ForgeCommit>> {
    info!("attempting to get commits for all packages at once");
    let mut starting_sha = None;
    let mut oldest_timestamp = i64::MAX;

    for package in packages.iter() {
        let tag_prefix = get_tag_prefix(package, repo_name);

        if let Some(tag) =
            forge_manager.get_latest_tag_for_prefix(&tag_prefix).await?
            && let Some(timestamp) = tag.timestamp
        {
            if timestamp < oldest_timestamp {
                oldest_timestamp = timestamp;
                starting_sha = Some(tag.sha);
            }
        } else {
            // since we have a package that hasn't been tagged yet, we can't
            // determine if oldest tag for other packages will sufficiently
            // capture all the necessary commits for this package so we
            // must fall back on pull individually for each package
            warn!("found package that hasn't been tagged yet");
            starting_sha = None;
            break;
        }
    }

    if starting_sha.is_none() {
        warn!("falling back to getting commits for each package separately");
        return get_commits_for_all_packages_separately(
            forge_manager,
            packages,
            repo_name,
        )
        .await;
    }

    info!("getting commits");
    forge_manager.get_commits(starting_sha).await
}

/// Filters list of commit to just the commits pertaining to a specific package
pub fn filter_commits_for_package(
    package: &PackageConfig,
    tag: Option<Tag>,
    commits: &[ForgeCommit],
) -> Vec<ForgeCommit> {
    let package_full_path = package_path(package, None);

    let mut package_paths = vec![package_full_path];

    if let Some(additional_paths) = package.additional_paths.clone() {
        package_paths.extend(additional_paths);
    }

    let mut package_commits: Vec<ForgeCommit> = vec![];

    for commit in commits.iter() {
        if let Some(tag) = tag.clone()
            && let Some(tag_timestamp) = tag.timestamp
            && commit.timestamp < tag_timestamp
        {
            // commit is older than package's previous release starting point
            continue;
        }
        'file_loop: for file in commit.files.iter() {
            let file_path = Path::new(file);
            for package_path in package_paths.iter() {
                let normalized_path =
                    package_path.replace("\\", "/").replace("./", "");
                let mut normalized_path = Path::new(&normalized_path);
                if package_path == "." {
                    normalized_path = Path::new("");
                }
                if file_path.starts_with(normalized_path) {
                    let raw_message = commit.message.to_string();
                    let split_msg = raw_message
                        .split_once("\n")
                        .map(|(m, b)| (m.to_string(), b.to_string()));

                    let (title, _body) = match split_msg {
                        Some((t, b)) => {
                            if b.is_empty() {
                                (t.trim().to_string(), None)
                            } else {
                                (
                                    t.trim().to_string(),
                                    Some(b.trim().to_string()),
                                )
                            }
                        }
                        None => (raw_message.to_string(), None),
                    };

                    debug!(
                        "{}: including commit : {} : {}",
                        package.name, commit.short_id, title
                    );

                    package_commits.push(commit.clone());
                    break 'file_loop;
                }
            }
        }
    }

    package_commits
}

/// When we can't determine a common starting point for all packages, we fall
/// back to pulling commits for each package individually and dedup by storing
/// in a HashSet
async fn get_commits_for_all_packages_separately(
    forge_manager: &ForgeManager,
    packages: &[PackageConfig],
    repo_name: &str,
) -> Result<Vec<ForgeCommit>> {
    let mut cache: HashSet<ForgeCommit> = HashSet::new();

    for package in packages.iter() {
        let tag_prefix = get_tag_prefix(package, repo_name);

        let current_tag =
            forge_manager.get_latest_tag_for_prefix(&tag_prefix).await?;

        let current_sha = current_tag.clone().map(|t| t.sha);

        info!(
            "{}: current tag sha: {:?} : fetching commits",
            package.name, current_sha
        );

        let commits = forge_manager.get_commits(current_sha).await?;

        cache.extend(commits);
    }

    let mut commits = cache.iter().cloned().collect::<Vec<ForgeCommit>>();

    commits.sort_by(|c1, c2| c1.timestamp.cmp(&c2.timestamp));

    Ok(commits)
}

#[cfg(test)]
mod tests {
    use crate::{config::release_type::ReleaseType, test_helpers};

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
            prerelease_version: None,
            additional_paths: None,
            additional_manifest_files: None,
            breaking_always_increment_major: Some(true),
            features_always_increment_minor: Some(true),
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
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
            prerelease_version: None,
            additional_paths: None,
            additional_manifest_files: None,
            breaking_always_increment_major: Some(true),
            features_always_increment_minor: Some(true),
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
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
            prerelease_version: None,
            additional_paths: None,
            additional_manifest_files: None,
            breaking_always_increment_major: Some(true),
            features_always_increment_minor: Some(true),
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
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
            prerelease_version: None,
            additional_paths: None,
            additional_manifest_files: None,
            breaking_always_increment_major: Some(true),
            features_always_increment_minor: Some(true),
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
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
            prerelease_version: None,
            additional_paths: None,
            additional_manifest_files: None,
            breaking_always_increment_major: Some(true),
            features_always_increment_minor: Some(true),
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
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
                prerelease_version: None,
                additional_paths: None,
                additional_manifest_files: None,
                breaking_always_increment_major: Some(true),
                features_always_increment_minor: Some(true),
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
            },
            PackageConfig {
                name: "".into(),
                path: "packages/api".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: None,
                prerelease: None,
                prerelease_version: None,
                additional_paths: None,
                additional_manifest_files: None,
                breaking_always_increment_major: Some(true),
                features_always_increment_minor: Some(true),
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
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
                prerelease_version: None,
                additional_paths: None,
                additional_manifest_files: None,
                breaking_always_increment_major: Some(true),
                features_always_increment_minor: Some(true),
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
            },
            PackageConfig {
                name: "another-name".into(),
                path: "packages/api".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: None,
                prerelease: None,
                prerelease_version: None,
                additional_paths: None,
                additional_manifest_files: None,
                breaking_always_increment_major: Some(true),
                features_always_increment_minor: Some(true),
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
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
                prerelease_version: None,
                additional_paths: None,
                additional_manifest_files: None,
                breaking_always_increment_major: Some(true),
                features_always_increment_minor: Some(true),
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
            },
            PackageConfig {
                name: "".into(),
                path: "packages/backend".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: None,
                prerelease: None,
                prerelease_version: None,
                additional_paths: None,
                additional_manifest_files: None,
                breaking_always_increment_major: Some(true),
                features_always_increment_minor: Some(true),
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
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
            prerelease_version: None,
            additional_paths: None,
            additional_manifest_files: None,
            breaking_always_increment_major: Some(true),
            features_always_increment_minor: Some(true),
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
        };

        let name = derive_package_name(&package, "test-repo");
        assert_eq!(name, "explicit-package-name");
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
                prerelease_version: None,
                additional_paths: None,
                additional_manifest_files: None,
                breaking_always_increment_major: Some(true),
                features_always_increment_minor: Some(true),
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
            }]);
        // Set both global and package
        config.prerelease = Some("alpha".to_string());
        config.packages[0].prerelease = Some("beta".to_string());

        let result = get_prerelease(&config, &config.packages[0]);

        // Package should win over global
        assert_eq!(result, Some("beta".to_string()));
    }

    #[test]
    fn test_get_prerelease_version_package_overrides_global() {
        let mut config =
            test_helpers::create_test_config(vec![PackageConfig {
                name: "my-package".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Generic),
                tag_prefix: None,
                prerelease: None,
                prerelease_version: Some(true),
                additional_paths: None,
                additional_manifest_files: None,
                breaking_always_increment_major: Some(true),
                features_always_increment_minor: Some(true),
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
            }]);
        // Set both global and package
        config.prerelease = Some("alpha".to_string());
        config.prerelease_version = true;
        config.packages[0].prerelease = Some("beta".to_string());
        config.packages[0].prerelease_version = Some(false);

        let result = get_prerelease_version(&config, &config.packages[0]);

        // Package should win over global
        assert_eq!(result, false);
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
                prerelease_version: None,
                additional_paths: None,
                additional_manifest_files: None,
                breaking_always_increment_major: Some(true),
                features_always_increment_minor: Some(true),
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
            }]);
        // Set only global
        config.prerelease = Some("alpha".to_string());

        let result = get_prerelease(&config, &config.packages[0]);

        // Should use global
        assert_eq!(result, Some("alpha".to_string()));
    }

    #[test]
    fn test_get_prerelease_version_uses_global_when_package_not_set() {
        let mut config =
            test_helpers::create_test_config(vec![PackageConfig {
                name: "my-package".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Generic),
                tag_prefix: None,
                prerelease: None,
                prerelease_version: None,
                additional_paths: None,
                additional_manifest_files: None,
                breaking_always_increment_major: Some(true),
                features_always_increment_minor: Some(true),
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
            }]);
        // Set only global
        config.prerelease = Some("alpha".to_string());
        config.prerelease_version = false;

        let result = get_prerelease_version(&config, &config.packages[0]);

        // Should use global
        assert_eq!(result, false);
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
            prerelease_version: None,
            additional_paths: None,
            additional_manifest_files: None,
            breaking_always_increment_major: Some(true),
            features_always_increment_minor: Some(true),
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
        }]);
        // Nothing set

        let result = get_prerelease(&config, &config.packages[0]);

        // Should return None
        assert_eq!(result, None);
    }

    #[test]
    fn test_filter_commits_for_package_filters_by_tag_timestamp() {
        let package = PackageConfig {
            name: "my-package".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            prerelease_version: None,
            additional_paths: None,
            additional_manifest_files: None,
            breaking_always_increment_major: Some(true),
            features_always_increment_minor: Some(true),
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
        };

        // Create tag with timestamp 2000
        let mut tag =
            test_helpers::create_test_tag("v1.0.0", "1.0.0", "tag-sha");
        tag.timestamp = Some(2000);

        // Create commits with various timestamps
        let mut old_commit = test_helpers::create_test_forge_commit(
            "old-commit",
            "feat: old feature",
            1000, // Before tag
        );
        old_commit.files = vec!["src/main.rs".to_string()];

        let mut equal_commit = test_helpers::create_test_forge_commit(
            "equal-commit",
            "feat: equal feature",
            2000, // Equal to tag
        );
        equal_commit.files = vec!["src/lib.rs".to_string()];

        let mut new_commit = test_helpers::create_test_forge_commit(
            "new-commit",
            "feat: new feature",
            3000, // After tag
        );
        new_commit.files = vec!["src/utils.rs".to_string()];

        let commits = vec![old_commit, equal_commit, new_commit];

        // Filter with tag - should exclude commits older than tag timestamp
        let result = filter_commits_for_package(&package, Some(tag), &commits);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "equal-commit");
        assert_eq!(result[1].id, "new-commit");
    }

    #[test]
    fn test_generate_analyzer_config_uses_global_defaults() {
        let config = test_helpers::create_test_config(vec![PackageConfig {
            name: "test-pkg".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Node),
            tag_prefix: None,
            prerelease: None,
            prerelease_version: None,
            additional_paths: None,
            additional_manifest_files: None,
            breaking_always_increment_major: None,
            features_always_increment_minor: None,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
        }]);

        let remote_config = test_helpers::create_test_remote_config();
        let analyzer_config = generate_analyzer_config(
            &config,
            &remote_config,
            "main",
            &config.packages[0],
            "v".to_string(),
        );

        assert!(analyzer_config.breaking_always_increment_major);
        assert!(analyzer_config.features_always_increment_minor);
        assert_eq!(analyzer_config.custom_major_increment_regex, None);
        assert_eq!(analyzer_config.custom_minor_increment_regex, None);
    }

    #[test]
    fn test_generate_analyzer_config_package_overrides_boolean_flags() {
        let mut config =
            test_helpers::create_test_config(vec![PackageConfig {
                name: "test-pkg".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: None,
                prerelease: None,
                prerelease_version: None,
                additional_paths: None,
                additional_manifest_files: None,
                breaking_always_increment_major: Some(false),
                features_always_increment_minor: Some(false),
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
            }]);

        // Global config has defaults true
        config.breaking_always_increment_major = true;
        config.features_always_increment_minor = true;

        let remote_config = test_helpers::create_test_remote_config();
        let analyzer_config = generate_analyzer_config(
            &config,
            &remote_config,
            "main",
            &config.packages[0],
            "v".to_string(),
        );

        // Package config should override global
        assert!(!analyzer_config.breaking_always_increment_major);
        assert!(!analyzer_config.features_always_increment_minor);
    }

    #[test]
    fn test_generate_analyzer_config_package_overrides_custom_regex() {
        let mut config =
            test_helpers::create_test_config(vec![PackageConfig {
                name: "test-pkg".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: None,
                prerelease: None,
                prerelease_version: None,
                additional_paths: None,
                additional_manifest_files: None,
                breaking_always_increment_major: None,
                features_always_increment_minor: None,
                custom_major_increment_regex: Some("PKG_MAJOR".to_string()),
                custom_minor_increment_regex: Some("PKG_MINOR".to_string()),
            }]);

        // Set global custom regex
        config.custom_major_increment_regex = Some("GLOBAL_MAJOR".to_string());
        config.custom_minor_increment_regex = Some("GLOBAL_MINOR".to_string());

        let remote_config = test_helpers::create_test_remote_config();
        let analyzer_config = generate_analyzer_config(
            &config,
            &remote_config,
            "main",
            &config.packages[0],
            "v".to_string(),
        );

        // Package config should override global
        assert_eq!(
            analyzer_config.custom_major_increment_regex,
            Some("PKG_MAJOR".to_string())
        );
        assert_eq!(
            analyzer_config.custom_minor_increment_regex,
            Some("PKG_MINOR".to_string())
        );
    }

    #[test]
    fn test_generate_analyzer_config_uses_global_when_package_not_set() {
        let mut config =
            test_helpers::create_test_config(vec![PackageConfig {
                name: "test-pkg".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: None,
                prerelease: None,
                prerelease_version: None,
                additional_paths: None,
                additional_manifest_files: None,
                breaking_always_increment_major: None,
                features_always_increment_minor: None,
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
            }]);

        // Set only global config
        config.breaking_always_increment_major = false;
        config.custom_major_increment_regex = Some("MAJOR".to_string());

        let remote_config = test_helpers::create_test_remote_config();
        let analyzer_config = generate_analyzer_config(
            &config,
            &remote_config,
            "main",
            &config.packages[0],
            "v".to_string(),
        );

        // Should use global config
        assert!(!analyzer_config.breaking_always_increment_major);
        assert_eq!(
            analyzer_config.custom_major_increment_regex,
            Some("MAJOR".to_string())
        );
    }
}
