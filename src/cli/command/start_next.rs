//! Starts next release by creating chore commits to bump patch versions in
//! manifest files for each package
use log::*;

use crate::{
    Result, cli::common, config::Config, forge::manager::ForgeManager,
};

/// Perform patch version update on manifest files and commit as "chore" to
/// start the next release cycle
pub async fn execute(
    forge_manager: &ForgeManager,
    targets: Option<Vec<String>>,
    config: Config,
) -> Result<()> {
    let base_branch = config.base_branch()?;

    let target_packages = if let Some(targets) = targets {
        config
            .packages
            .iter()
            .filter(|p| targets.contains(&p.name))
            .cloned()
            .collect()
    } else {
        config.packages.clone()
    };

    let mut tagged_packages = vec![];

    for p in target_packages.iter() {
        let tag_prefix = p.tag_prefix()?;
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

    common::start_next_release(&config.packages, forge_manager, &base_branch)
        .await
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
        let config = Config {
            base_branch: Some("main".into()),
            packages: vec![PackageConfig {
                name: "test-pkg".into(),
                path: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: Some("v".to_string()),
                ..PackageConfig::default()
            }],
            ..Config::default()
        };

        let mut mock = MockForge::new();

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_remote_config().returning(RemoteConfig::default);

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

        execute(&forge_manager, None, config).await.unwrap();
    }

    #[tokio::test]
    async fn skips_packages_without_tags() {
        let config = Config {
            base_branch: Some("main".into()),
            packages: vec![PackageConfig {
                name: "untagged-pkg".into(),
                path: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: Some("v".to_string()),
                ..PackageConfig::default()
            }],
            ..Config::default()
        };

        let mut mock = MockForge::new();

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_remote_config().returning(RemoteConfig::default);

        mock.expect_get_latest_tag_for_prefix()
            .returning(|_| Ok(None));

        mock.expect_get_file_content().returning(|_| Ok(None));

        mock.expect_create_commit().returning(|_| {
            Ok(Commit {
                sha: "new-commit-sha".into(),
            })
        });

        let forge_manager = ForgeManager::new(Box::new(mock));

        execute(&forge_manager, None, config).await.unwrap()
    }

    #[tokio::test]
    async fn handles_multiple_packages() {
        let config = Config {
            base_branch: Some("main".into()),
            packages: vec![
                PackageConfig {
                    name: "pkg-a".into(),
                    path: "./packages/a".into(),
                    release_type: Some(ReleaseType::Node),
                    tag_prefix: Some("pkg-a-v".to_string()),
                    ..PackageConfig::default()
                },
                PackageConfig {
                    name: "pkg-b".into(),
                    path: "./packages/b".into(),
                    release_type: Some(ReleaseType::Rust),
                    tag_prefix: Some("pkg-b-v".to_string()),
                    ..PackageConfig::default()
                },
            ],
            ..Config::default()
        };

        let mut mock = MockForge::new();

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_remote_config().returning(RemoteConfig::default);

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

        execute(&forge_manager, None, config).await.unwrap();
    }

    #[tokio::test]
    async fn uses_base_branch_override() {
        let config = Config {
            base_branch: Some("develop".into()),
            packages: vec![],
            ..Config::default()
        };

        let mut mock = MockForge::new();

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_remote_config().returning(RemoteConfig::default);

        let forge_manager = ForgeManager::new(Box::new(mock));

        execute(&forge_manager, None, config).await.unwrap();
    }

    #[tokio::test]
    async fn succeeds_with_empty_package_list() {
        let config = Config {
            base_branch: Some("main".into()),
            packages: vec![],
            ..Config::default()
        };

        let mut mock = MockForge::new();

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_remote_config().returning(RemoteConfig::default);

        let forge_manager = ForgeManager::new(Box::new(mock));

        execute(&forge_manager, None, config).await.unwrap();
    }

    #[tokio::test]
    async fn filters_to_specific_packages() {
        let config = Config {
            base_branch: Some("main".into()),
            packages: vec![
                PackageConfig {
                    name: "pkg-a".into(),
                    path: "./packages/a".into(),
                    release_type: Some(ReleaseType::Node),
                    tag_prefix: Some("pkg-a-v".to_string()),
                    ..PackageConfig::default()
                },
                PackageConfig {
                    name: "pkg-b".into(),
                    path: "./packages/b".into(),
                    release_type: Some(ReleaseType::Rust),
                    tag_prefix: Some("pkg-b-v".to_string()),
                    ..PackageConfig::default()
                },
            ],
            ..Config::default()
        };

        let mut mock = MockForge::new();

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_remote_config().returning(RemoteConfig::default);

        // Only expect calls for pkg-a
        mock.expect_get_latest_tag_for_prefix()
            .times(3)
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
            .times(2)
            .withf(|req| req.message.contains("chore(main): bump"))
            .returning(|_| {
                Ok(Commit {
                    sha: "new-commit-sha".into(),
                })
            });

        let forge_manager = ForgeManager::new(Box::new(mock));

        execute(&forge_manager, Some(vec!["pkg-a".into()]), config)
            .await
            .unwrap()
    }
}
