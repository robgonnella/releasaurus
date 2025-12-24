//! Starts next release by creating chore commits to bump patch versions in
//! manifest files for each package
use log::*;

use crate::{Result, cli::common, forge::manager::ForgeManager};

/// Perform patch version update on manifest files and commit as "chore" to
/// start the next release cycle
pub async fn execute(
    forge_manager: &ForgeManager,
    targets: Option<Vec<String>>,
    base_branch_override: Option<String>,
) -> Result<()> {
    let repo_name = forge_manager.repo_name();

    let mut config = forge_manager
        .load_config(base_branch_override.clone())
        .await?;

    config = common::process_config(&repo_name, &mut config);

    if let Some(targets) = targets {
        config.packages.retain(|p| targets.contains(&p.name));
    }

    let mut tagged_packages = vec![];

    for p in config.packages.iter() {
        let tag_prefix = common::get_tag_prefix(p, &repo_name);
        let current =
            forge_manager.get_latest_tag_for_prefix(&tag_prefix).await?;
        if current.is_none() {
            warn!(
                "package {} has not been tagged yet: cannot start-next: skipping",
                p.name
            );
            continue;
        }

        tagged_packages.push(p.clone());
    }

    config.packages = tagged_packages;

    let base_branch =
        common::base_branch(&config, forge_manager, base_branch_override);

    common::start_next_release(&config, forge_manager, &base_branch).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::release::Tag,
        config::{Config, package::PackageConfig, release_type::ReleaseType},
        forge::{config::RemoteConfig, request::Commit, traits::MockForge},
    };
    use semver::Version as SemVer;

    #[tokio::test]
    async fn succeeds_when_package_has_tag() {
        let mut mock = MockForge::new();

        mock.expect_load_config().returning(|_| {
            Ok(Config {
                packages: vec![PackageConfig {
                    name: "test-pkg".into(),
                    path: ".".into(),
                    release_type: Some(ReleaseType::Node),
                    ..PackageConfig::default()
                }],
                ..Config::default()
            })
        });

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_remote_config().returning(RemoteConfig::default);

        mock.expect_default_branch()
            .returning(|| "main".to_string());

        mock.expect_get_latest_tag_for_prefix().returning(|_| {
            Ok(Some(Tag {
                name: "v1.0.0".into(),
                semver: SemVer::parse("1.0.0").unwrap(),
                sha: "abc123".into(),
                ..Tag::default()
            }))
        });

        mock.expect_get_file_content().returning(|_req| Ok(None));

        mock.expect_create_commit().returning(|_req| {
            Ok(Commit {
                sha: "new-commit-sha".into(),
            })
        });

        let forge_manager = ForgeManager::new(Box::new(mock));

        let result = execute(&forge_manager, None, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn skips_packages_without_tags() {
        let mut mock = MockForge::new();

        mock.expect_load_config().returning(|_| {
            Ok(Config {
                packages: vec![PackageConfig {
                    name: "untagged-pkg".into(),
                    path: ".".into(),
                    release_type: Some(ReleaseType::Node),
                    ..PackageConfig::default()
                }],
                ..Config::default()
            })
        });

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_remote_config().returning(RemoteConfig::default);

        mock.expect_default_branch()
            .returning(|| "main".to_string());

        mock.expect_get_latest_tag_for_prefix()
            .returning(|_| Ok(None));

        let forge_manager = ForgeManager::new(Box::new(mock));

        execute(&forge_manager, None, None).await.unwrap()
    }

    #[tokio::test]
    async fn handles_multiple_packages() {
        let mut mock = MockForge::new();

        mock.expect_load_config().returning(|_| {
            Ok(Config {
                packages: vec![
                    PackageConfig {
                        name: "pkg-a".into(),
                        path: "./packages/a".into(),
                        release_type: Some(ReleaseType::Node),
                        ..PackageConfig::default()
                    },
                    PackageConfig {
                        name: "pkg-b".into(),
                        path: "./packages/b".into(),
                        release_type: Some(ReleaseType::Rust),
                        ..PackageConfig::default()
                    },
                ],
                ..Config::default()
            })
        });

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_remote_config().returning(RemoteConfig::default);

        mock.expect_default_branch()
            .returning(|| "main".to_string());

        mock.expect_get_latest_tag_for_prefix().returning(|prefix| {
            if prefix.contains("pkg-a") {
                Ok(Some(Tag {
                    name: "pkg-a-v1.0.0".into(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                    sha: "abc123".into(),
                    ..Tag::default()
                }))
            } else {
                Ok(Some(Tag {
                    name: "pkg-b-v2.0.0".into(),
                    semver: SemVer::parse("2.0.0").unwrap(),
                    sha: "def456".into(),
                    ..Tag::default()
                }))
            }
        });

        mock.expect_get_file_content().returning(|_req| Ok(None));

        mock.expect_create_commit().returning(|_req| {
            Ok(Commit {
                sha: "new-commit-sha".into(),
            })
        });

        let forge_manager = ForgeManager::new(Box::new(mock));

        let result = execute(&forge_manager, None, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn uses_base_branch_override() {
        let mut mock = MockForge::new();

        mock.expect_load_config().returning(|branch| {
            assert_eq!(branch, Some("develop".to_string()));
            Ok(Config {
                packages: vec![],
                ..Config::default()
            })
        });

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_remote_config().returning(RemoteConfig::default);

        mock.expect_default_branch()
            .times(0)
            .returning(|| "main".to_string());

        let forge_manager = ForgeManager::new(Box::new(mock));

        let result =
            execute(&forge_manager, None, Some("develop".into())).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn succeeds_with_empty_package_list() {
        let mut mock = MockForge::new();

        mock.expect_load_config().returning(|_| {
            Ok(Config {
                packages: vec![],
                ..Config::default()
            })
        });

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_remote_config().returning(RemoteConfig::default);

        mock.expect_default_branch()
            .returning(|| "main".to_string());

        let forge_manager = ForgeManager::new(Box::new(mock));

        let result = execute(&forge_manager, None, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn filters_to_specific_packages() {
        let mut mock = MockForge::new();

        mock.expect_load_config().returning(|_| {
            Ok(Config {
                packages: vec![
                    PackageConfig {
                        name: "pkg-a".into(),
                        path: "./packages/a".into(),
                        release_type: Some(ReleaseType::Node),
                        ..PackageConfig::default()
                    },
                    PackageConfig {
                        name: "pkg-b".into(),
                        path: "./packages/b".into(),
                        release_type: Some(ReleaseType::Rust),
                        ..PackageConfig::default()
                    },
                ],
                ..Config::default()
            })
        });

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_remote_config().returning(RemoteConfig::default);

        mock.expect_default_branch()
            .returning(|| "main".to_string());

        // Only expect calls for pkg-a
        mock.expect_get_latest_tag_for_prefix()
            .times(2)
            .returning(|req| {
                if req.contains("pkg-a") {
                    Ok(Some(Tag {
                        name: "pkg-a-v1.0.0".into(),
                        semver: SemVer::parse("1.0.0").unwrap(),
                        sha: "abc123".into(),
                        ..Tag::default()
                    }))
                } else {
                    Ok(Some(Tag {
                        name: "pkg-b-v1.0.0".into(),
                        semver: SemVer::parse("1.0.0").unwrap(),
                        sha: "def456".into(),
                        ..Tag::default()
                    }))
                }
            });

        mock.expect_get_file_content().returning(|_| Ok(None));

        mock.expect_create_commit()
            .times(1)
            .withf(|req| req.message.contains("chore(main): bump"))
            .returning(|_| {
                Ok(Commit {
                    sha: "new-commit-sha".into(),
                })
            });

        let forge_manager = ForgeManager::new(Box::new(mock));

        execute(&forge_manager, Some(vec!["pkg-a".into()]), None)
            .await
            .unwrap()
    }
}
