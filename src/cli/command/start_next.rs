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
            .into_iter()
            .filter(|p| targets.contains(&p.name))
            .collect()
    } else {
        config.packages
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

    common::start_next_release(&tagged_packages, forge_manager, &base_branch)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::release::Tag,
        config::{
            Config, package::PackageConfigBuilder, release_type::ReleaseType,
        },
        forge::{request::Commit, traits::MockForge},
    };
    use semver::Version as SemVer;

    #[tokio::test]
    async fn succeeds_when_package_has_tag() {
        let config = Config {
            base_branch: Some("main".into()),
            packages: vec![
                PackageConfigBuilder::default()
                    .name("test-pkg")
                    .path(".")
                    .release_type(ReleaseType::Node)
                    .tag_prefix("v")
                    .build()
                    .unwrap(),
            ],
            ..Config::default()
        };

        let mut mock = MockForge::new();

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_dry_run().returning(|| false);

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
            packages: vec![
                PackageConfigBuilder::default()
                    .name("untagged-pkg")
                    .path(".")
                    .release_type(ReleaseType::Node)
                    .tag_prefix("v")
                    .build()
                    .unwrap(),
            ],
            ..Config::default()
        };

        let mut mock = MockForge::new();

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_dry_run().returning(|| false);

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
                PackageConfigBuilder::default()
                    .name("pkg-a")
                    .path("./packages/a")
                    .release_type(ReleaseType::Node)
                    .tag_prefix("pkg-a-v")
                    .build()
                    .unwrap(),
                PackageConfigBuilder::default()
                    .name("pkg-b")
                    .path("./packages/b")
                    .release_type(ReleaseType::Rust)
                    .tag_prefix("pkg-b-v")
                    .build()
                    .unwrap(),
            ],
            ..Config::default()
        };

        let mut mock = MockForge::new();

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_dry_run().returning(|| false);

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

        mock.expect_dry_run().returning(|| false);

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

        mock.expect_dry_run().returning(|| false);

        let forge_manager = ForgeManager::new(Box::new(mock));

        execute(&forge_manager, None, config).await.unwrap();
    }

    #[tokio::test]
    async fn filters_to_specific_packages() {
        let config = Config {
            base_branch: Some("main".into()),
            packages: vec![
                PackageConfigBuilder::default()
                    .name("pkg-a")
                    .path("./packages/a")
                    .release_type(ReleaseType::Node)
                    .tag_prefix("pkg-a-v")
                    .build()
                    .unwrap(),
                PackageConfigBuilder::default()
                    .name("pkg-b")
                    .path("./packages/b")
                    .release_type(ReleaseType::Rust)
                    .tag_prefix("pkg-b-v")
                    .build()
                    .unwrap(),
            ],
            ..Config::default()
        };

        let mut mock = MockForge::new();

        mock.expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock.expect_dry_run().returning(|| false);

        // Only expect calls for pkg-a (not pkg-b since we filtered)
        mock.expect_get_latest_tag_for_prefix()
            .times(2)
            .withf(|req| req.contains("pkg-a"))
            .returning(|_| {
                Ok(Some(Tag {
                    name: "pkg-a-v1.0.0".into(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                    sha: "abc123".into(),
                    ..Tag::default()
                }))
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

        execute(&forge_manager, Some(vec!["pkg-a".into()]), config)
            .await
            .unwrap()
    }
}
