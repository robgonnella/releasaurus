//! Final release publication and tagging command implementation.
use color_eyre::eyre::OptionExt;
use log::*;
use regex::Regex;
use std::sync::LazyLock;

use crate::{
    Result,
    cli::common::{self, PRMetadata},
    config::{Config, package::PackageConfig},
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, TAGGED_LABEL},
        manager::ForgeManager,
        request::{GetPrRequest, PrLabelsRequest, PullRequest},
    },
};

static METADATA_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?ms)^<!--(?<metadata>.*?)-->\n*<details"#).unwrap()
});

/// Execute release command by finding the merged release PR, tagging commits,
/// and publishing releases to the forge platform.
pub async fn execute(
    forge_manager: &ForgeManager,
    config: Config,
) -> Result<()> {
    let base_branch = config.base_branch()?;

    let mut auto_start_packages: Vec<String> = vec![];

    for package in config.packages.iter() {
        let mut release_branch =
            format!("{DEFAULT_PR_BRANCH_PREFIX}-{base_branch}");

        if config.separate_pull_requests {
            release_branch = format!(
                "{DEFAULT_PR_BRANCH_PREFIX}-{base_branch}-{}",
                package.name
            );
        }

        let req = GetPrRequest {
            base_branch: base_branch.clone(),
            head_branch: release_branch.to_string(),
        };

        if let Some(merged_pr) =
            forge_manager.get_merged_release_pr(req).await?
        {
            create_package_release(forge_manager, package, &merged_pr).await?;

            let req = PrLabelsRequest {
                pr_number: merged_pr.number,
                labels: vec![TAGGED_LABEL.into()],
            };

            forge_manager.replace_pr_labels(req).await?;

            let auto_start_next = config.auto_start_next(package);

            if auto_start_next {
                auto_start_packages.push(package.name.clone());
            };
        }
    }

    if !auto_start_packages.is_empty() {
        let filtered_packages: Vec<PackageConfig> = config
            .packages
            .iter()
            .filter(|p| auto_start_packages.contains(&p.name))
            .cloned()
            .collect();

        common::start_next_release(
            &filtered_packages,
            forge_manager,
            &base_branch,
        )
        .await?;
    }

    Ok(())
}

/// Creates release for a targeted package and merged PR
async fn create_package_release(
    forge_manager: &ForgeManager,
    package: &PackageConfig,
    merged_pr: &PullRequest,
) -> Result<()> {
    let meta_caps = METADATA_REGEX.captures_iter(&merged_pr.body);

    let mut metadata = None;

    for cap in meta_caps {
        let metadata_str = cap
            .name("metadata")
            .ok_or_eyre("failed to parse metadata from PR body")?
            .as_str();

        debug!("parsing metadata string: {:#?}", metadata_str);

        let json: PRMetadata = serde_json::from_str(metadata_str)?;
        let pkg_meta = json.metadata;

        if pkg_meta.name == package.name {
            metadata = Some(pkg_meta);
            break;
        }
    }

    let metadata_err = format!(
        "failed to find metadata for package {} in pr {}",
        package.name, merged_pr.number,
    );

    let metadata = metadata.ok_or_eyre(metadata_err)?;

    debug!(
        "found package metadata from pr {}: {:#?}",
        merged_pr.number, metadata
    );

    info!(
        "tagging commit: tag: {}, sha: {}",
        metadata.tag, merged_pr.sha
    );

    forge_manager
        .tag_commit(&metadata.tag, &merged_pr.sha)
        .await?;

    info!(
        "creating release: tag: {}, sha: {}",
        metadata.tag, merged_pr.sha
    );

    forge_manager
        .create_release(&metadata.tag, &merged_pr.sha, metadata.notes.trim())
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{
            Config, ConfigBuilder, package::PackageConfigBuilder,
            release_type::ReleaseType,
        },
        forge::{config::RemoteConfig, traits::MockForge},
    };

    #[tokio::test]
    async fn test_execute_creates_release_for_merged_pr() {
        let config = ConfigBuilder::default()
            .base_branch("main")
            .packages(vec![
                PackageConfigBuilder::default()
                    .name("my-package")
                    .release_type(ReleaseType::Node)
                    .tag_prefix("v")
                    .build()
                    .unwrap(),
            ])
            .build()
            .unwrap();

        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_merged_release_pr()
            .withf(|req| {
                req.base_branch == "main"
                    && req.head_branch == "releasaurus-release-main"
            })
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 42,
                    sha: "abc123".to_string(),
                    body: "<!--{\"metadata\":{\"name\":\"my-package\",\"tag\":\"v1.0.0\",\"notes\":\"## Changes\\n\\n- feat: new feature\"}}-->\n<details>".to_string(),
                }))
            });

        mock_forge
            .expect_tag_commit()
            .withf(|tag, sha| tag == "v1.0.0" && sha == "abc123")
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag, sha, notes| {
                tag == "v1.0.0"
                    && sha == "abc123"
                    && notes == "## Changes\n\n- feat: new feature"
            })
            .returning(|_, _, _| Ok(()));

        mock_forge
            .expect_replace_pr_labels()
            .withf(|req| {
                req.pr_number == 42 && req.labels == vec!["releasaurus:tagged"]
            })
            .returning(|_| Ok(()));

        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        let manager = ForgeManager::new(Box::new(mock_forge));

        execute(&manager, config).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_handles_separate_pull_requests() {
        let config = ConfigBuilder::default()
            .base_branch("main")
            .separate_pull_requests(true)
            .packages(vec![
                PackageConfigBuilder::default()
                    .name("pkg-a")
                    .release_type(ReleaseType::Node)
                    .tag_prefix("pkg-a-v")
                    .build()
                    .unwrap(),
            ])
            .build()
            .unwrap();

        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_merged_release_pr()
            .withf(|req| {
                req.base_branch == "main"
                    && req.head_branch == "releasaurus-release-main-pkg-a"
            })
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 10,
                    sha: "def456".to_string(),
                    body: "<!--{\"metadata\":{\"name\":\"pkg-a\",\"tag\":\"pkg-a-v2.0.0\",\"notes\":\"Breaking changes\"}}-->\n<details>".to_string(),
                }))
            });

        mock_forge.expect_tag_commit().returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .returning(|_, _, _| Ok(()));

        mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        let manager = ForgeManager::new(Box::new(mock_forge));

        execute(&manager, config).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_skips_packages_without_merged_pr() {
        let config = ConfigBuilder::default()
            .base_branch("main")
            .packages(vec![
                PackageConfigBuilder::default()
                    .name("my-package")
                    .release_type(ReleaseType::Node)
                    .tag_prefix("v")
                    .build()
                    .unwrap(),
            ])
            .build()
            .unwrap();

        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_merged_release_pr()
            .returning(|_| Ok(None));

        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        let manager = ForgeManager::new(Box::new(mock_forge));

        // Should not call tag_commit, create_release, or replace_pr_labels

        execute(&manager, config).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_processes_multiple_packages() {
        let config = ConfigBuilder::default()
            .base_branch("main")
            .packages(vec![
                PackageConfigBuilder::default()
                    .name("pkg-a")
                    .path("packages/a")
                    .workspace_root(".")
                    .release_type(ReleaseType::Node)
                    .tag_prefix("pkg-a-v")
                    .build()
                    .unwrap(),
                PackageConfigBuilder::default()
                    .name("pkg-b")
                    .path("packages/b")
                    .workspace_root(".")
                    .release_type(ReleaseType::Rust)
                    .tag_prefix("pkg-b-v")
                    .build()
                    .unwrap(),
            ])
            .build()
            .unwrap();

        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_merged_release_pr()
            .times(2)
            .returning(|req| {
                if req.head_branch == "releasaurus-release-main" {
                    Ok(Some(PullRequest {
                        number: 1,
                        sha: "sha1".to_string(),
                        body: "<!--{\"metadata\":{\"name\":\"pkg-a\",\"tag\":\"pkg-a-v1.0.0\",\"notes\":\"Release pkg-a\"}}-->\n<details>\n<!--{\"metadata\":{\"name\":\"pkg-b\",\"tag\":\"pkg-b-v2.0.0\",\"notes\":\"Release pkg-b\"}}-->\n<details>".to_string(),
                    }))
                } else {
                    Ok(None)
                }
            });

        mock_forge
            .expect_tag_commit()
            .times(2)
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .times(2)
            .returning(|_, _, _| Ok(()));

        mock_forge
            .expect_replace_pr_labels()
            .times(2)
            .returning(|_| Ok(()));

        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        let manager = ForgeManager::new(Box::new(mock_forge));

        execute(&manager, config).await.unwrap();
    }

    #[tokio::test]
    async fn test_create_package_release_matches_correct_package_by_name() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_tag_commit()
            .withf(|tag, sha| tag == "pkg-b-v2.0.0" && sha == "test-sha")
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag, sha, notes| {
                tag == "pkg-b-v2.0.0"
                    && sha == "test-sha"
                    && notes == "Release notes for pkg-b"
            })
            .returning(|_, _, _| Ok(()));

        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        let forge_manger = ForgeManager::new(Box::new(mock_forge));

        let package = PackageConfigBuilder::default()
            .name("pkg-b")
            .path("packages/b")
            .release_type(ReleaseType::Rust)
            .tag_prefix("pkg-b-v")
            .build()
            .unwrap();

        let pr = PullRequest {
            number: 42,
            sha: "test-sha".to_string(),
            body: "<!--{\"metadata\":{\"name\":\"pkg-a\",\"tag\":\"pkg-a-v1.0.0\",\"notes\":\"Release notes for pkg-a\"}}-->\n<details>\n<!--{\"metadata\":{\"name\":\"pkg-b\",\"tag\":\"pkg-b-v2.0.0\",\"notes\":\"Release notes for pkg-b\"}}-->\n<details>".to_string(),
        };

        create_package_release(&forge_manger, &package, &pr)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_create_package_release_trims_notes() {
        let mut mock_forge = MockForge::new();

        mock_forge.expect_tag_commit().returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|_, _, notes| notes == "Trimmed notes")
            .returning(|_, _, _| Ok(()));

        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        let manager = ForgeManager::new(Box::new(mock_forge));

        let package = PackageConfigBuilder::default()
            .name("my-package")
            .release_type(ReleaseType::Node)
            .tag_prefix("v")
            .build()
            .unwrap();

        let pr = PullRequest {
            number: 42,
            sha: "test-sha".to_string(),
            body: "<!--{\"metadata\":{\"name\":\"my-package\",\"tag\":\"v1.0.0\",\"notes\":\"  Trimmed notes  \\n  \"}}-->\n<details>".to_string(),
        };

        create_package_release(&manager, &package, &pr)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_create_package_release_fails_when_metadata_missing() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        let manager = ForgeManager::new(Box::new(mock_forge));

        let package = PackageConfigBuilder::default()
            .name("my-package")
            .release_type(ReleaseType::Node)
            .tag_prefix("v")
            .build()
            .unwrap();

        let pr = PullRequest {
            number: 42,
            sha: "test-sha".to_string(),
            body: "No metadata here".to_string(),
        };

        let result = create_package_release(&manager, &package, &pr).await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("failed to find metadata")
        );
    }

    #[tokio::test]
    async fn test_create_package_release_fails_when_metadata_malformed() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        let manager = ForgeManager::new(Box::new(mock_forge));

        let package = PackageConfigBuilder::default()
            .name("my-package")
            .release_type(ReleaseType::Node)
            .tag_prefix("v")
            .build()
            .unwrap();

        let pr = PullRequest {
            number: 42,
            sha: "test-sha".to_string(),
            body: "<!--{\"invalid json\"}-->\n<details>".to_string(),
        };

        let result = create_package_release(&manager, &package, &pr).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_package_release_fails_when_package_name_not_found() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        let manager = ForgeManager::new(Box::new(mock_forge));

        let package = PackageConfigBuilder::default()
            .name("my-package")
            .release_type(ReleaseType::Node)
            .tag_prefix("v")
            .build()
            .unwrap();

        let pr = PullRequest {
            number: 42,
            sha: "test-sha".to_string(),
            body: "<!--{\"metadata\":{\"name\":\"other-package\",\"tag\":\"v1.0.0\",\"notes\":\"Release notes\"}}-->\n<details>".to_string(),
        };

        let result = create_package_release(&manager, &package, &pr).await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("failed to find metadata for package my-package")
        );
    }

    #[tokio::test]
    async fn test_execute_multiple_packages_single_pr() {
        let config = Config {
            base_branch: Some("main".into()),
            packages: vec![
                PackageConfigBuilder::default()
                    .name("api")
                    .path("packages/api")
                    .workspace_root(".")
                    .release_type(ReleaseType::Node)
                    .tag_prefix("api-v")
                    .build()
                    .unwrap(),
                PackageConfigBuilder::default()
                    .name("web")
                    .path("packages/web")
                    .workspace_root(".")
                    .release_type(ReleaseType::Rust)
                    .tag_prefix("web-v")
                    .build()
                    .unwrap(),
            ],
            ..Config::default()
        };

        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_merged_release_pr()
            .times(2)
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 99,
                    sha: "multi-sha".to_string(),
                    body: "<!--{\"metadata\":{\"name\":\"api\",\"tag\":\"api-v1.2.0\",\"notes\":\"## API Changes\\n\\n- feat: new endpoint\"}}-->\n<details>\n<summary>api v1.2.0</summary>\nAPI release details\n</details>\n\n<!--{\"metadata\":{\"name\":\"web\",\"tag\":\"web-v2.5.0\",\"notes\":\"## Web Changes\\n\\n- fix: ui bug\"}}-->\n<details>\n<summary>web v2.5.0</summary>\nWeb release details\n</details>".to_string(),
                }))
            });

        mock_forge
            .expect_tag_commit()
            .withf(|tag, sha| {
                (tag == "api-v1.2.0" || tag == "web-v2.5.0")
                    && sha == "multi-sha"
            })
            .times(2)
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag, sha, notes| {
                sha == "multi-sha"
                    && ((tag == "api-v1.2.0"
                        && notes == "## API Changes\n\n- feat: new endpoint")
                        || (tag == "web-v2.5.0"
                            && notes == "## Web Changes\n\n- fix: ui bug"))
            })
            .times(2)
            .returning(|_, _, _| Ok(()));

        mock_forge
            .expect_replace_pr_labels()
            .withf(|req| {
                req.pr_number == 99 && req.labels == vec!["releasaurus:tagged"]
            })
            .times(2)
            .returning(|_| Ok(()));

        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        let manager = ForgeManager::new(Box::new(mock_forge));

        execute(&manager, config).await.unwrap();
    }

    #[tokio::test]
    async fn test_uses_config_base_branch_override() {
        let config = ConfigBuilder::default()
            .base_branch("develop")
            .packages(vec![
                PackageConfigBuilder::default()
                    .name("my-package")
                    .build()
                    .unwrap(),
            ])
            .build()
            .unwrap();

        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_default_branch()
            .times(0)
            .returning(|| "main".to_string());

        mock_forge
                .expect_get_merged_release_pr()
            .withf(|req| {
                req.base_branch == "develop"
                    && req.head_branch == "releasaurus-release-develop"
            })
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 42,
                    sha: "abc123".to_string(),
                    body: "<!--{\"metadata\":{\"name\":\"my-package\",\"tag\":\"v1.0.0\",\"notes\":\"## Changes\\n\\n- feat: new feature\"}}-->\n<details>".to_string(),
                }))
            });

        mock_forge.expect_tag_commit().returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .returning(|_, _, _| Ok(()));

        mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        let manager = ForgeManager::new(Box::new(mock_forge));

        execute(&manager, config).await.unwrap();
    }

    #[tokio::test]
    async fn test_uses_cli_base_branch_override() {
        let config = ConfigBuilder::default()
            .base_branch("develop")
            .packages(vec![
                PackageConfigBuilder::default()
                    .name("my-package")
                    .release_type(ReleaseType::Node)
                    .tag_prefix("v")
                    .build()
                    .unwrap(),
            ])
            .build()
            .unwrap();

        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_default_branch()
            .times(0)
            .returning(|| "main".to_string());

        mock_forge
                .expect_get_merged_release_pr()
            .withf(|req| {
                req.base_branch == "develop"
                    && req.head_branch == "releasaurus-release-develop"
            })
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 42,
                    sha: "abc123".to_string(),
                    body: "<!--{\"metadata\":{\"name\":\"my-package\",\"tag\":\"v1.0.0\",\"notes\":\"## Changes\\n\\n- feat: new feature\"}}-->\n<details>".to_string(),
                }))
            });

        mock_forge.expect_tag_commit().returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .returning(|_, _, _| Ok(()));

        mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        let manager = ForgeManager::new(Box::new(mock_forge));

        execute(&manager, config).await.unwrap();
    }

    #[tokio::test]
    async fn test_auto_start_next_enabled_globally() {
        let config = ConfigBuilder::default()
            .base_branch("main")
            .auto_start_next(true)
            .packages(vec![
                PackageConfigBuilder::default()
                    .name("my-package")
                    .release_type(ReleaseType::Node)
                    .tag_prefix("v")
                    .build()
                    .unwrap(),
            ])
            .build()
            .unwrap();

        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_merged_release_pr()
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 42,
                    sha: "abc123".to_string(),
                    body: "<!--{\"metadata\":{\"name\":\"my-package\",\"tag\":\"v1.0.0\",\"notes\":\"Release\"}}-->\n<details>".to_string(),
                }))
            });

        mock_forge.expect_tag_commit().returning(|_, _| Ok(()));
        mock_forge
            .expect_create_release()
            .returning(|_, _, _| Ok(()));
        mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));
        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        // Expect start_next to be called
        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(crate::analyzer::release::Tag {
                    name: "v1.0.0".into(),
                    semver: semver::Version::parse("1.0.0").unwrap(),
                    ..Default::default()
                }))
            });

        mock_forge.expect_get_file_content().returning(|_| Ok(None));

        mock_forge.expect_create_commit().returning(|_| {
            Ok(crate::forge::request::Commit {
                sha: "new-sha".into(),
            })
        });

        let manager = ForgeManager::new(Box::new(mock_forge));

        execute(&manager, config).await.unwrap();
    }

    #[tokio::test]
    async fn test_auto_start_next_enabled_at_package_level() {
        let config = Config {
            base_branch: Some("main".into()),
            auto_start_next: Some(false),
            packages: vec![
                PackageConfigBuilder::default()
                    .name("pkg1")
                    .release_type(ReleaseType::Node)
                    .auto_start_next(true)
                    .tag_prefix("v")
                    .build()
                    .unwrap(),
                PackageConfigBuilder::default()
                    .name("pkg2")
                    .release_type(ReleaseType::Node)
                    .tag_prefix("v")
                    .build()
                    .unwrap(),
            ],
            ..Config::default()
        };

        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_merged_release_pr()
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 42,
                    sha: "abc123".to_string(),
                    body: "<!--{\"metadata\":{\"name\":\"pkg1\",\"tag\":\"v1.0.0\",\"notes\":\"Release\"}}-->\n<details>\n\n<!--{\"metadata\":{\"name\":\"pkg2\",\"tag\":\"v1.0.0\",\"notes\":\"Release\"}}-->\n<details>".to_string(),
                }))
            });

        mock_forge.expect_tag_commit().returning(|_, _| Ok(()));
        mock_forge
            .expect_create_release()
            .returning(|_, _, _| Ok(()));
        mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));
        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(crate::analyzer::release::Tag {
                    name: "v1.0.0".into(),
                    semver: semver::Version::parse("1.0.0").unwrap(),
                    ..Default::default()
                }))
            });

        mock_forge.expect_get_file_content().returning(|_| Ok(None));

        mock_forge.expect_create_commit().returning(|_| {
            Ok(crate::forge::request::Commit {
                sha: "new-sha".into(),
            })
        });

        let manager = ForgeManager::new(Box::new(mock_forge));

        execute(&manager, config).await.unwrap();
    }

    #[tokio::test]
    async fn test_auto_start_next_not_called_when_disabled() {
        let config = ConfigBuilder::default()
            .base_branch("main")
            .packages(vec![
                PackageConfigBuilder::default()
                    .name("my-package")
                    .release_type(ReleaseType::Node)
                    .build()
                    .unwrap(),
            ])
            .build()
            .unwrap();

        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_merged_release_pr()
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 42,
                    sha: "abc123".to_string(),
                    body: "<!--{\"metadata\":{\"name\":\"my-package\",\"tag\":\"v1.0.0\",\"notes\":\"Release\"}}-->\n<details>".to_string(),
                }))
            });

        mock_forge.expect_tag_commit().returning(|_, _| Ok(()));
        mock_forge
            .expect_create_release()
            .returning(|_, _, _| Ok(()));
        mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));
        mock_forge
            .expect_remote_config()
            .returning(RemoteConfig::default);

        // Should NOT call start_next functions
        mock_forge.expect_get_latest_tag_for_prefix().times(0);
        mock_forge.expect_create_commit().times(0);

        let manager = ForgeManager::new(Box::new(mock_forge));

        execute(&manager, config).await.unwrap();
    }
}
