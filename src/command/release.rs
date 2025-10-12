//! Final release publication and tagging command implementation.
use log::*;

use crate::{
    analyzer::Analyzer,
    command::common,
    config::{Config, PackageConfig},
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, RemoteConfig, TAGGED_LABEL},
        request::{GetPrRequest, PrLabelsRequest, PullRequest},
        traits::Forge,
    },
    result::Result,
};

/// Execute release command by finding the merged release PR, tagging commits,
/// and publishing releases to the forge platform.
pub async fn execute(
    forge: Box<dyn Forge>,
    prerelease_override: Option<String>,
) -> Result<()> {
    let repo_name = forge.repo_name();
    let mut config = forge.load_config().await?;
    let config = common::process_config(&repo_name, &mut config);
    let default_branch = forge.default_branch().await?;

    for package in config.packages.iter() {
        let mut release_branch =
            format!("{DEFAULT_PR_BRANCH_PREFIX}-{default_branch}");

        if config.separate_pull_requests {
            release_branch = format!(
                "{DEFAULT_PR_BRANCH_PREFIX}-{default_branch}-{}",
                package.name
            );
        }

        generate_branch_release(
            forge.as_ref(),
            package,
            &release_branch,
            &config,
            prerelease_override.clone(),
        )
        .await?;
    }

    Ok(())
}

async fn generate_branch_release(
    forge: &dyn Forge,
    package: &PackageConfig,
    release_branch: &str,
    config: &Config,
    prerelease_override: Option<String>,
) -> Result<()> {
    let default_branch = forge.default_branch().await?;
    let remote_config = forge.remote_config();

    let req = GetPrRequest {
        base_branch: default_branch.clone(),
        head_branch: release_branch.to_string(),
    };

    let merged_pr = forge.get_merged_release_pr(req).await?;

    if merged_pr.is_none() {
        warn!(
            "releases are up-to-date for package {} and branch {release_branch}: nothing to release",
            package.name,
        );
        return Ok(());
    }

    let merged_pr = merged_pr.unwrap();

    create_package_release(
        config,
        &remote_config,
        forge,
        &merged_pr,
        package,
        prerelease_override,
    )
    .await?;

    let req = PrLabelsRequest {
        pr_number: merged_pr.number,
        labels: vec![TAGGED_LABEL.into()],
    };

    forge.replace_pr_labels(req).await?;

    Ok(())
}

/// Analyze commits since last tag, determine next version, create git tag, and
/// publish release with generated notes.
async fn create_package_release(
    config: &Config,
    remote_config: &RemoteConfig,
    forge: &dyn Forge,
    merged_pr: &PullRequest,
    package: &PackageConfig,
    prerelease_override: Option<String>,
) -> Result<()> {
    let repo_name = forge.repo_name();
    let tag_prefix = common::get_tag_prefix(package, &repo_name);
    let current_tag = forge.get_latest_tag_for_prefix(&tag_prefix).await?;
    let current_sha = current_tag.clone().map(|t| t.sha);
    let commits = forge.get_commits(&package.path, current_sha).await?;

    // Determine prerelease with priority: CLI override > package config > global config
    let prerelease =
        common::get_prerelease(config, package, prerelease_override);

    let analyzer_config = common::generate_analyzer_config(
        config,
        remote_config,
        package,
        tag_prefix,
        prerelease,
    );

    let analyzer = Analyzer::new(analyzer_config)?;
    let release = analyzer.analyze(commits, current_tag)?;

    if let Some(release) = release
        && let Some(tag) = release.tag.clone()
    {
        forge.tag_commit(&tag.name, &merged_pr.sha).await?;
        forge
            .create_release(&tag.name, &release.sha, &release.notes)
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::release::Tag,
        config::ReleaseType,
        forge::{
            request::{ForgeCommit, PullRequest},
            traits::MockForge,
        },
        test_helpers,
    };
    use semver::Version as SemVer;

    #[tokio::test]
    async fn test_generate_branch_release_no_merged_pr() {
        let config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "my-package",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_remote_config()
            .times(1)
            .returning(test_helpers::create_test_remote_config);

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .withf(|req| {
                req.base_branch == "main"
                    && req.head_branch == "releasaurus-release-main"
            })
            .returning(|_| Ok(None));

        let result = generate_branch_release(
            &mock_forge,
            &config.packages[0],
            "releasaurus-release-main",
            &config,
            None,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_generate_branch_release_with_merged_pr_no_release() {
        let config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "my-package",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_remote_config()
            .times(1)
            .returning(test_helpers::create_test_remote_config);

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 42,
                    sha: "merged-pr-sha".to_string(),
                }))
            });

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .withf(|prefix| prefix == "v")
            .returning(|_| {
                Ok(Some(Tag {
                    sha: "tag-sha".to_string(),
                    name: "v1.0.0".to_string(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                }))
            });

        mock_forge
            .expect_get_commits()
            .times(1)
            .withf(|path, sha| {
                path == "." && sha == &Some("tag-sha".to_string())
            })
            .returning(|_, _| Ok(vec![]));

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .withf(|req| {
                req.pr_number == 42 && req.labels == vec!["releasaurus:tagged"]
            })
            .returning(|_| Ok(()));

        let result = generate_branch_release(
            &mock_forge,
            &config.packages[0],
            "releasaurus-release-main",
            &config,
            None,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_generate_branch_release_creates_tag_and_release() {
        let config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "my-package",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_remote_config()
            .times(1)
            .returning(test_helpers::create_test_remote_config);

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 42,
                    sha: "merged-pr-sha".to_string(),
                }))
            });

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .withf(|prefix| prefix == "v")
            .returning(|_| {
                Ok(Some(Tag {
                    sha: "old-tag-sha".to_string(),
                    name: "v1.0.0".to_string(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                }))
            });

        mock_forge.expect_get_commits().times(1).returning(|_, _| {
            Ok(vec![ForgeCommit {
                id: "commit1".to_string(),
                link: "https://github.com/test/repo/commit/commit1".to_string(),
                author_name: "Test Author".to_string(),
                author_email: "test@example.com".to_string(),
                merge_commit: false,
                message: "feat: new feature".to_string(),
                timestamp: 1000,
            }])
        });

        mock_forge
            .expect_tag_commit()
            .times(1)
            .withf(|tag_name, sha| {
                tag_name == "v1.1.0" && sha == "merged-pr-sha"
            })
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .times(1)
            .withf(|tag_name, _sha, _notes| tag_name == "v1.1.0")
            .returning(|_, _, _| Ok(()));

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .withf(|req| req.pr_number == 42)
            .returning(|_| Ok(()));

        let result = generate_branch_release(
            &mock_forge,
            &config.packages[0],
            "releasaurus-release-main",
            &config,
            None,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_first_release() {
        let config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "my-package",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);

        let remote_config = test_helpers::create_test_remote_config();
        let merged_pr = PullRequest {
            number: 42,
            sha: "merged-pr-sha".to_string(),
        };

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .withf(|prefix| prefix == "v")
            .returning(|_| Ok(None));

        mock_forge
            .expect_get_commits()
            .times(1)
            .withf(|path, sha| path == "." && sha.is_none())
            .returning(|_, _| {
                Ok(vec![ForgeCommit {
                    id: "commit1".to_string(),
                    link: "https://github.com/test/repo/commit/commit1"
                        .to_string(),
                    author_name: "Test Author".to_string(),
                    author_email: "test@example.com".to_string(),
                    merge_commit: false,
                    message: "feat: initial release".to_string(),
                    timestamp: 1000,
                }])
            });

        mock_forge
            .expect_tag_commit()
            .times(1)
            .withf(|tag_name, sha| {
                tag_name == "v0.1.0" && sha == "merged-pr-sha"
            })
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .times(1)
            .withf(|tag_name, _sha, _notes| tag_name == "v0.1.0")
            .returning(|_, _, _| Ok(()));

        let result = create_package_release(
            &config,
            &remote_config,
            &mock_forge,
            &merged_pr,
            &config.packages[0],
            None,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_no_changes() {
        let config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "my-package",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);

        let remote_config = test_helpers::create_test_remote_config();
        let merged_pr = PullRequest {
            number: 42,
            sha: "merged-pr-sha".to_string(),
        };

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| {
                Ok(Some(Tag {
                    sha: "tag-sha".to_string(),
                    name: "v1.0.0".to_string(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                }))
            });

        mock_forge
            .expect_get_commits()
            .times(1)
            .returning(|_, _| Ok(vec![]));

        // No tag_commit or create_release should be called

        let result = create_package_release(
            &config,
            &remote_config,
            &mock_forge,
            &merged_pr,
            &config.packages[0],
            None,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_with_custom_tag_prefix() {
        let config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "api-package",
                "packages/api",
                Some(ReleaseType::Generic),
                Some("api-v".to_string()),
            ),
        ]);

        let remote_config = test_helpers::create_test_remote_config();
        let merged_pr = PullRequest {
            number: 42,
            sha: "merged-pr-sha".to_string(),
        };

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .withf(|prefix| prefix == "api-v")
            .returning(|_| {
                Ok(Some(Tag {
                    sha: "old-tag-sha".to_string(),
                    name: "api-v1.0.0".to_string(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                }))
            });

        mock_forge
            .expect_get_commits()
            .times(1)
            .withf(|path, _| path == "packages/api")
            .returning(|_, _| {
                Ok(vec![ForgeCommit {
                    id: "commit1".to_string(),
                    link: "https://github.com/test/repo/commit/commit1"
                        .to_string(),
                    author_name: "Test Author".to_string(),
                    author_email: "test@example.com".to_string(),
                    merge_commit: false,
                    message: "fix: bug fix".to_string(),
                    timestamp: 1000,
                }])
            });

        mock_forge
            .expect_tag_commit()
            .times(1)
            .withf(|tag_name, sha| {
                tag_name == "api-v1.0.1" && sha == "merged-pr-sha"
            })
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .times(1)
            .withf(|tag_name, _sha, _notes| tag_name == "api-v1.0.1")
            .returning(|_, _, _| Ok(()));

        let result = create_package_release(
            &config,
            &remote_config,
            &mock_forge,
            &merged_pr,
            &config.packages[0],
            None,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_major_version_bump() {
        let config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "my-package",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);

        let remote_config = test_helpers::create_test_remote_config();
        let merged_pr = PullRequest {
            number: 42,
            sha: "merged-pr-sha".to_string(),
        };

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| {
                Ok(Some(Tag {
                    sha: "tag-sha".to_string(),
                    name: "v1.5.3".to_string(),
                    semver: SemVer::parse("1.5.3").unwrap(),
                }))
            });

        mock_forge.expect_get_commits().times(1).returning(|_, _| {
            Ok(vec![ForgeCommit {
                id: "commit1".to_string(),
                link: "https://github.com/test/repo/commit/commit1".to_string(),
                author_name: "Test Author".to_string(),
                author_email: "test@example.com".to_string(),
                merge_commit: false,
                message: "feat!: breaking change".to_string(),
                timestamp: 1000,
            }])
        });

        mock_forge
            .expect_tag_commit()
            .times(1)
            .withf(|tag_name, sha| {
                tag_name == "v2.0.0" && sha == "merged-pr-sha"
            })
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .times(1)
            .withf(|tag_name, _sha, _notes| tag_name == "v2.0.0")
            .returning(|_, _, _| Ok(()));

        let result = create_package_release(
            &config,
            &remote_config,
            &mock_forge,
            &merged_pr,
            &config.packages[0],
            None,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_with_prerelease_from_package_config() {
        let mut config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "my-package",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);
        // Set prerelease on package
        config.packages[0].prerelease = Some("alpha".to_string());

        let remote_config = test_helpers::create_test_remote_config();
        let merged_pr = PullRequest {
            number: 42,
            sha: "merged-pr-sha".to_string(),
        };

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| {
                Ok(Some(Tag {
                    sha: "tag-sha".to_string(),
                    name: "v1.0.0".to_string(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                }))
            });

        mock_forge.expect_get_commits().times(1).returning(|_, _| {
            Ok(vec![ForgeCommit {
                id: "commit1".to_string(),
                link: "https://github.com/test/repo/commit/commit1".to_string(),
                author_name: "Test Author".to_string(),
                author_email: "test@example.com".to_string(),
                merge_commit: false,
                message: "feat: new feature".to_string(),
                timestamp: 1000,
            }])
        });

        mock_forge
            .expect_tag_commit()
            .times(1)
            .withf(|tag_name, sha| {
                tag_name == "v1.1.0-alpha.1" && sha == "merged-pr-sha"
            })
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .times(1)
            .withf(|tag_name, _sha, _notes| tag_name == "v1.1.0-alpha.1")
            .returning(|_, _, _| Ok(()));

        let result = create_package_release(
            &config,
            &remote_config,
            &mock_forge,
            &merged_pr,
            &config.packages[0],
            None,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_with_prerelease_from_global_config() {
        let mut config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "my-package",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);
        // Set global prerelease
        config.prerelease = Some("beta".to_string());

        let remote_config = test_helpers::create_test_remote_config();
        let merged_pr = PullRequest {
            number: 42,
            sha: "merged-pr-sha".to_string(),
        };

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| {
                Ok(Some(Tag {
                    sha: "tag-sha".to_string(),
                    name: "v2.0.0".to_string(),
                    semver: SemVer::parse("2.0.0").unwrap(),
                }))
            });

        mock_forge.expect_get_commits().times(1).returning(|_, _| {
            Ok(vec![ForgeCommit {
                id: "commit1".to_string(),
                link: "https://github.com/test/repo/commit/commit1".to_string(),
                author_name: "Test Author".to_string(),
                author_email: "test@example.com".to_string(),
                merge_commit: false,
                message: "fix: bug fix".to_string(),
                timestamp: 1000,
            }])
        });

        mock_forge
            .expect_tag_commit()
            .times(1)
            .withf(|tag_name, sha| {
                tag_name == "v2.0.1-beta.1" && sha == "merged-pr-sha"
            })
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .times(1)
            .withf(|tag_name, _sha, _notes| tag_name == "v2.0.1-beta.1")
            .returning(|_, _, _| Ok(()));

        let result = create_package_release(
            &config,
            &remote_config,
            &mock_forge,
            &merged_pr,
            &config.packages[0],
            None,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_prerelease_package_overrides_global() {
        let mut config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "my-package",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);
        // Set both global and package prerelease - package should win
        config.prerelease = Some("alpha".to_string());
        config.packages[0].prerelease = Some("rc".to_string());

        let remote_config = test_helpers::create_test_remote_config();
        let merged_pr = PullRequest {
            number: 42,
            sha: "merged-pr-sha".to_string(),
        };

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| {
                Ok(Some(Tag {
                    sha: "tag-sha".to_string(),
                    name: "v0.5.0".to_string(),
                    semver: SemVer::parse("0.5.0").unwrap(),
                }))
            });

        mock_forge.expect_get_commits().times(1).returning(|_, _| {
            Ok(vec![ForgeCommit {
                id: "commit1".to_string(),
                link: "https://github.com/test/repo/commit/commit1".to_string(),
                author_name: "Test Author".to_string(),
                author_email: "test@example.com".to_string(),
                merge_commit: false,
                message: "feat: new feature".to_string(),
                timestamp: 1000,
            }])
        });

        mock_forge
            .expect_tag_commit()
            .times(1)
            .withf(|tag_name, sha| {
                // Should use "rc" not "alpha"
                // Feature commit on 0.5.0 bumps to 0.6.0 in 0.x versions
                tag_name == "v0.6.0-rc.1" && sha == "merged-pr-sha"
            })
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .times(1)
            .withf(|tag_name, _sha, _notes| tag_name == "v0.6.0-rc.1")
            .returning(|_, _, _| Ok(()));

        let result = create_package_release(
            &config,
            &remote_config,
            &mock_forge,
            &merged_pr,
            &config.packages[0],
            None,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_cli_override_takes_priority() {
        let mut config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "my-package",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);
        // Set all three levels - CLI should win
        config.prerelease = Some("alpha".to_string());
        config.packages[0].prerelease = Some("beta".to_string());
        let cli_override = Some("dev".to_string());

        let remote_config = test_helpers::create_test_remote_config();
        let merged_pr = PullRequest {
            number: 42,
            sha: "merged-pr-sha".to_string(),
        };

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| {
                Ok(Some(Tag {
                    sha: "tag-sha".to_string(),
                    name: "v1.0.0".to_string(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                }))
            });

        mock_forge.expect_get_commits().times(1).returning(|_, _| {
            Ok(vec![ForgeCommit {
                id: "commit1".to_string(),
                link: "https://github.com/test/repo/commit/commit1".to_string(),
                author_name: "Test Author".to_string(),
                author_email: "test@example.com".to_string(),
                merge_commit: false,
                message: "feat: new feature".to_string(),
                timestamp: 1000,
            }])
        });

        mock_forge
            .expect_tag_commit()
            .times(1)
            .withf(|tag_name, sha| {
                // Should use "dev" from CLI, not "alpha" or "beta"
                tag_name == "v1.1.0-dev.1" && sha == "merged-pr-sha"
            })
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .times(1)
            .withf(|tag_name, _sha, _notes| tag_name == "v1.1.0-dev.1")
            .returning(|_, _, _| Ok(()));

        let result = create_package_release(
            &config,
            &remote_config,
            &mock_forge,
            &merged_pr,
            &config.packages[0],
            cli_override,
        )
        .await;

        assert!(result.is_ok());
    }
}
