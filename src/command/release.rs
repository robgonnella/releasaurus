//! Final release publication and tagging command implementation.
use color_eyre::eyre::OptionExt;
use log::*;
use regex::Regex;
use std::sync::LazyLock;

use crate::{
    Result,
    command::common::{self, PRMetadata},
    config::package::PackageConfig,
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
pub async fn execute(forge_manager: &ForgeManager) -> Result<()> {
    let repo_name = forge_manager.repo_name();
    let mut config = forge_manager.load_config().await?;
    let config = common::process_config(&repo_name, &mut config);
    let default_branch = forge_manager.default_branch();

    for package in config.packages.iter() {
        let mut release_branch =
            format!("{DEFAULT_PR_BRANCH_PREFIX}-{default_branch}");

        if config.separate_pull_requests {
            release_branch = format!(
                "{DEFAULT_PR_BRANCH_PREFIX}-{default_branch}-{}",
                package.name
            );
        }

        let default_branch = forge_manager.default_branch();

        let req = GetPrRequest {
            base_branch: default_branch.clone(),
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
        }
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
        config::release_type::ReleaseType, forge::traits::MockForge,
        test_helpers::*,
    };

    #[tokio::test]
    async fn test_execute_creates_release_for_merged_pr() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge.expect_load_config().returning(|| {
            Ok(create_test_config(vec![PackageConfig {
                name: "my-package".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: Some("v".to_string()),
                prerelease: None,
                prerelease_version: None,
                breaking_always_increment_major: None,
                features_always_increment_minor: None,
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
                additional_paths: None,
                additional_manifest_files: None,
            }]))
        });

        mock_forge
            .expect_default_branch()
            .times(2)
            .returning(|| "main".to_string());

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
            .returning(create_test_remote_config);

        let manager = ForgeManager::new(Box::new(mock_forge));

        let result = execute(&manager).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_handles_separate_pull_requests() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge.expect_load_config().returning(|| {
            let mut config = create_test_config(vec![PackageConfig {
                name: "pkg-a".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: Some("pkg-a-v".to_string()),
                prerelease: None,
                prerelease_version: None,
                breaking_always_increment_major: None,
                features_always_increment_minor: None,
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
                additional_paths: None,
                additional_manifest_files: None,
            }]);
            config.separate_pull_requests = true;
            Ok(config)
        });

        mock_forge
            .expect_default_branch()
            .times(2)
            .returning(|| "main".to_string());

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
            .returning(create_test_remote_config);

        let manager = ForgeManager::new(Box::new(mock_forge));

        let result = execute(&manager).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_skips_packages_without_merged_pr() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge.expect_load_config().returning(|| {
            Ok(create_test_config(vec![PackageConfig {
                name: "my-package".into(),
                path: ".".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Node),
                tag_prefix: Some("v".to_string()),
                prerelease: None,
                prerelease_version: None,
                breaking_always_increment_major: None,
                features_always_increment_minor: None,
                custom_major_increment_regex: None,
                custom_minor_increment_regex: None,
                additional_paths: None,
                additional_manifest_files: None,
            }]))
        });

        mock_forge
            .expect_default_branch()
            .times(2)
            .returning(|| "main".to_string());

        mock_forge
            .expect_get_merged_release_pr()
            .returning(|_| Ok(None));

        mock_forge
            .expect_remote_config()
            .returning(create_test_remote_config);

        let manager = ForgeManager::new(Box::new(mock_forge));

        // Should not call tag_commit, create_release, or replace_pr_labels

        let result = execute(&manager).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_processes_multiple_packages() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge.expect_load_config().returning(|| {
            Ok(create_test_config(vec![
                PackageConfig {
                    name: "pkg-a".into(),
                    path: "packages/a".into(),
                    workspace_root: ".".into(),
                    release_type: Some(ReleaseType::Node),
                    tag_prefix: Some("pkg-a-v".to_string()),
                    prerelease: None,
                    prerelease_version: None,
                    breaking_always_increment_major: None,
                    features_always_increment_minor: None,
                    custom_major_increment_regex: None,
                    custom_minor_increment_regex: None,
                    additional_paths: None,
                    additional_manifest_files: None,
                },
                PackageConfig {
                    name: "pkg-b".into(),
                    path: "packages/b".into(),
                    workspace_root: ".".into(),
                    release_type: Some(ReleaseType::Rust),
                    tag_prefix: Some("pkg-b-v".to_string()),
                    prerelease: None,
                    prerelease_version: None,
                    breaking_always_increment_major: None,
                    features_always_increment_minor: None,
                    custom_major_increment_regex: None,
                    custom_minor_increment_regex: None,
                    additional_paths: None,
                    additional_manifest_files: None,
                },
            ]))
        });

        mock_forge
            .expect_default_branch()
            .times(3)
            .returning(|| "main".to_string());

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
            .returning(create_test_remote_config);

        let manager = ForgeManager::new(Box::new(mock_forge));

        let result = execute(&manager).await;
        assert!(result.is_ok());
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
            .returning(create_test_remote_config);

        let forge_manger = ForgeManager::new(Box::new(mock_forge));

        let package = PackageConfig {
            name: "pkg-b".into(),
            path: "packages/b".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Rust),
            tag_prefix: Some("pkg-b-v".to_string()),
            prerelease: None,
            prerelease_version: None,
            breaking_always_increment_major: None,
            features_always_increment_minor: None,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            additional_paths: None,
            additional_manifest_files: None,
        };

        let pr = PullRequest {
            number: 42,
            sha: "test-sha".to_string(),
            body: "<!--{\"metadata\":{\"name\":\"pkg-a\",\"tag\":\"pkg-a-v1.0.0\",\"notes\":\"Release notes for pkg-a\"}}-->\n<details>\n<!--{\"metadata\":{\"name\":\"pkg-b\",\"tag\":\"pkg-b-v2.0.0\",\"notes\":\"Release notes for pkg-b\"}}-->\n<details>".to_string(),
        };

        let result = create_package_release(&forge_manger, &package, &pr).await;
        assert!(result.is_ok());
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
            .returning(create_test_remote_config);

        let manager = ForgeManager::new(Box::new(mock_forge));

        let package = PackageConfig {
            name: "my-package".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Node),
            tag_prefix: Some("v".to_string()),
            prerelease: None,
            prerelease_version: None,
            breaking_always_increment_major: None,
            features_always_increment_minor: None,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            additional_paths: None,
            additional_manifest_files: None,
        };

        let pr = PullRequest {
            number: 42,
            sha: "test-sha".to_string(),
            body: "<!--{\"metadata\":{\"name\":\"my-package\",\"tag\":\"v1.0.0\",\"notes\":\"  Trimmed notes  \\n  \"}}-->\n<details>".to_string(),
        };

        let result = create_package_release(&manager, &package, &pr).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_fails_when_metadata_missing() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_remote_config()
            .returning(create_test_remote_config);

        let manager = ForgeManager::new(Box::new(mock_forge));

        let package = PackageConfig {
            name: "my-package".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Node),
            tag_prefix: Some("v".to_string()),
            prerelease: None,
            prerelease_version: None,
            breaking_always_increment_major: None,
            features_always_increment_minor: None,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            additional_paths: None,
            additional_manifest_files: None,
        };

        let pr = PullRequest {
            number: 42,
            sha: "test-sha".to_string(),
            body: "No metadata here".to_string(),
        };

        let result = create_package_release(&manager, &package, &pr).await;
        assert!(result.is_err());
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
            .returning(create_test_remote_config);

        let manager = ForgeManager::new(Box::new(mock_forge));

        let package = PackageConfig {
            name: "my-package".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Node),
            tag_prefix: Some("v".to_string()),
            prerelease: None,
            prerelease_version: None,
            breaking_always_increment_major: None,
            features_always_increment_minor: None,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            additional_paths: None,
            additional_manifest_files: None,
        };

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
            .returning(create_test_remote_config);

        let manager = ForgeManager::new(Box::new(mock_forge));

        let package = PackageConfig {
            name: "my-package".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Node),
            tag_prefix: Some("v".to_string()),
            prerelease: None,
            prerelease_version: None,
            breaking_always_increment_major: None,
            features_always_increment_minor: None,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            additional_paths: None,
            additional_manifest_files: None,
        };

        let pr = PullRequest {
            number: 42,
            sha: "test-sha".to_string(),
            body: "<!--{\"metadata\":{\"name\":\"other-package\",\"tag\":\"v1.0.0\",\"notes\":\"Release notes\"}}-->\n<details>".to_string(),
        };

        let result = create_package_release(&manager, &package, &pr).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("failed to find metadata for package my-package")
        );
    }

    #[tokio::test]
    async fn test_execute_multiple_packages_single_pr() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge.expect_load_config().returning(|| {
            Ok(create_test_config(vec![
                PackageConfig {
                    name: "api".into(),
                    path: "packages/api".into(),
                    workspace_root: ".".into(),
                    release_type: Some(ReleaseType::Node),
                    tag_prefix: Some("api-v".to_string()),
                    prerelease: None,
                    prerelease_version: None,
                    breaking_always_increment_major: None,
                    features_always_increment_minor: None,
                    custom_major_increment_regex: None,
                    custom_minor_increment_regex: None,
                    additional_paths: None,
                    additional_manifest_files: None,
                },
                PackageConfig {
                    name: "web".into(),
                    path: "packages/web".into(),
                    workspace_root: ".".into(),
                    release_type: Some(ReleaseType::Node),
                    tag_prefix: Some("web-v".to_string()),
                    prerelease: None,
                    prerelease_version: None,
                    breaking_always_increment_major: None,
                    features_always_increment_minor: None,
                    custom_major_increment_regex: None,
                    custom_minor_increment_regex: None,
                    additional_paths: None,
                    additional_manifest_files: None,
                },
            ]))
        });

        mock_forge
            .expect_default_branch()
            .times(3)
            .returning(|| "main".to_string());

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
            .returning(create_test_remote_config);

        let manager = ForgeManager::new(Box::new(mock_forge));

        let result = execute(&manager).await;
        assert!(result.is_ok());
    }
}
