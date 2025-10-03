//! Release pull request creation command implementation.

use log::*;
use std::{collections::HashMap, path::Path};

use crate::{
    analyzer::{Analyzer, config::AnalyzerConfig, release::Release},
    command::common,
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, PENDING_LABEL},
        request::{
            CreateBranchRequest, CreatePrRequest, FileChange, FileUpdateType,
            GetPrRequest, PrLabelsRequest, UpdatePrRequest,
        },
        traits::{FileLoader, Forge},
    },
    result::Result,
    updater::manager::UpdaterManager,
};

/// Execute release-pr command to analyze commits and create release pull request.
pub async fn execute(
    forge: Box<dyn Forge>,
    file_loader: Box<dyn FileLoader>,
) -> Result<()> {
    let remote_config = forge.remote_config();
    let config = forge.load_config().await?;
    let mut manifest: HashMap<String, Release> = HashMap::new();

    for package in config.packages.iter() {
        let tag_prefix = common::get_tag_prefix(package);
        info!(
            "processing package: path: {}, tag_prefix: {}",
            package.path, tag_prefix
        );
        let current_tag = forge.get_latest_tag_for_prefix(&tag_prefix).await?;

        info!("path: {}, current tag {:#?}", package.path, current_tag);

        let current_sha = current_tag.clone().map(|t| t.sha);
        let commits = forge.get_commits(&package.path, current_sha).await?;

        info!("processing commits for package: {}", package.path);

        let analyzer_config = AnalyzerConfig {
            body: config.changelog.body.clone(),
            release_link_base_url: remote_config.release_link_base_url.clone(),
            tag_prefix: Some(tag_prefix),
        };

        let analyzer = Analyzer::new(analyzer_config)?;
        let release = analyzer.analyze(commits, current_tag)?;

        info!("package path: {}, release: {:#?}", package.path, release);

        if let Some(release) = release {
            manifest.insert(package.path.clone(), release);
        }
    }

    debug!("manifest: {:#?}", manifest);

    let default_branch = forge.default_branch().await?;
    let release_branch =
        format!("{DEFAULT_PR_BRANCH_PREFIX}-{}", default_branch);

    let include_title_version = config.packages.len() == 1;
    let mut title = format!("chore({}): release", default_branch);

    let mut file_updates: Vec<FileChange> = vec![];
    let mut body = vec![];
    let mut start_tag = "<details>";

    // auto-open dropdown if there's only one package
    if manifest.len() == 1 {
        start_tag = "<details open>";
    }

    for (path, release) in manifest.iter() {
        if let Some(tag) = release.tag.clone() {
            if include_title_version {
                title = format!("{title} {}", tag.name);
            }

            // create the drop down
            let drop_down = format!(
                "{start_tag}<summary>{}</summary>\n\n{}</details>",
                tag.name, release.notes
            );

            body.push(drop_down);

            file_updates.push(FileChange {
                content: format!("{}\n\n", release.notes),
                path: Path::new(path)
                    .join("CHANGELOG.md")
                    .display()
                    .to_string(),
                update_type: FileUpdateType::Prepend,
            });
        }
    }

    let repo_name = forge.repo_name();

    let mut update_manager = UpdaterManager::new(&repo_name);

    if let Some(changes) = update_manager
        .update_packages(&manifest, &config, file_loader.as_ref())
        .await?
    {
        file_updates.extend(changes);
    }

    if !manifest.is_empty() {
        info!("creating / updating release branch: {release_branch}");
        forge
            .create_release_branch(CreateBranchRequest {
                branch: release_branch.clone(),
                message: title.clone(),
                file_changes: file_updates,
            })
            .await?;

        info!("searching for existing pr for branch {release_branch}");
        let pr = forge
            .get_open_release_pr(GetPrRequest {
                head_branch: release_branch.clone(),
                base_branch: default_branch.clone(),
            })
            .await?;

        let pr = if let Some(pr) = pr {
            forge
                .update_pr(UpdatePrRequest {
                    pr_number: pr.number,
                    title,
                    body: body.join("\n"),
                })
                .await?;
            info!("updated existing release-pr: {}", pr.number);
            pr
        } else {
            let pr = forge
                .create_pr(CreatePrRequest {
                    head_branch: release_branch,
                    base_branch: default_branch,
                    title,
                    body: body.join("\n"),
                })
                .await?;
            info!("created release-pr: {}", pr.number);
            pr
        };

        info!("setting pr labels: {PENDING_LABEL}");

        forge
            .replace_pr_labels(PrLabelsRequest {
                pr_number: pr.number,
                labels: vec![PENDING_LABEL.into()],
            })
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ReleaseType,
        forge::{
            request::{Commit, PullRequest},
            traits::{MockFileLoader, MockForge},
        },
        test_helpers::*,
    };

    #[tokio::test]
    async fn test_execute_with_single_package_no_existing_tag() {
        let mut mock_forge = MockForge::new();
        let mut mock_file_loader = MockFileLoader::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .with(mockall::predicate::eq("v"))
            .times(1)
            .returning(|_| Ok(None));

        mock_forge
            .expect_get_commits()
            .with(mockall::predicate::eq("."), mockall::predicate::eq(None))
            .times(1)
            .returning(|_, _| {
                Ok(vec![create_test_forge_commit(
                    "abc123",
                    "feat: add new feature",
                    1000,
                )])
            });

        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge.expect_repo_name().return_const("test-repo");

        mock_file_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_release_branch()
            .times(1)
            .returning(|_| {
                Ok(Commit {
                    sha: "new-sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_create_pr().times(1).returning(|_| {
            Ok(PullRequest {
                number: 42,
                sha: "pr-sha".to_string(),
            })
        });

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|_| Ok(()));

        let result =
            execute(Box::new(mock_forge), Box::new(mock_file_loader)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_single_package_existing_tag() {
        let mut mock_forge = MockForge::new();
        let mut mock_file_loader = MockFileLoader::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();
        let existing_tag = create_test_tag("v1.0.0", "1.0.0", "old-sha");

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        let tag_clone = existing_tag.clone();
        mock_forge
            .expect_get_latest_tag_for_prefix()
            .with(mockall::predicate::eq("v"))
            .times(1)
            .returning(move |_| Ok(Some(tag_clone.clone())));

        mock_forge.expect_get_commits().times(1).returning(|_, _| {
            Ok(vec![create_test_forge_commit(
                "abc123",
                "fix: fix bug",
                2000,
            )])
        });

        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge.expect_repo_name().return_const("test-repo");

        mock_file_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_release_branch()
            .times(1)
            .returning(|_| {
                Ok(Commit {
                    sha: "new-sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_create_pr().times(1).returning(|_| {
            Ok(PullRequest {
                number: 43,
                sha: "pr-sha".to_string(),
            })
        });

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|_| Ok(()));

        let result =
            execute(Box::new(mock_forge), Box::new(mock_file_loader)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_multiple_packages() {
        let mut mock_forge = MockForge::new();
        let mut mock_file_loader = MockFileLoader::new();

        let config = create_test_config(vec![
            create_test_package_config(
                "packages/frontend",
                Some(ReleaseType::Node),
                Some("frontend-v".to_string()),
            ),
            create_test_package_config(
                "packages/backend",
                Some(ReleaseType::Rust),
                Some("backend-v".to_string()),
            ),
        ]);

        let remote_config = create_test_remote_config();

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .with(mockall::predicate::eq("frontend-v"))
            .times(1)
            .returning(|_| Ok(None));

        mock_forge
            .expect_get_commits()
            .with(
                mockall::predicate::eq("packages/frontend"),
                mockall::predicate::eq(None),
            )
            .times(1)
            .returning(|_, _| {
                Ok(vec![create_test_forge_commit(
                    "abc123",
                    "feat: add frontend feature",
                    1000,
                )])
            });

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .with(mockall::predicate::eq("backend-v"))
            .times(1)
            .returning(|_| Ok(None));

        mock_forge
            .expect_get_commits()
            .with(
                mockall::predicate::eq("packages/backend"),
                mockall::predicate::eq(None),
            )
            .times(1)
            .returning(|_, _| {
                Ok(vec![create_test_forge_commit(
                    "def456",
                    "feat: add backend feature",
                    2000,
                )])
            });

        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge.expect_repo_name().return_const("test-repo");

        mock_file_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_release_branch()
            .times(1)
            .returning(|_| {
                Ok(Commit {
                    sha: "new-sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_create_pr().times(1).returning(|req| {
            assert!(!req.title.contains("v0.1.0"));
            assert_eq!(req.title, "chore(main): release");
            Ok(PullRequest {
                number: 44,
                sha: "pr-sha".to_string(),
            })
        });

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|_| Ok(()));

        let result =
            execute(Box::new(mock_forge), Box::new(mock_file_loader)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_updates_existing_pr() {
        let mut mock_forge = MockForge::new();
        let mut mock_file_loader = MockFileLoader::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_get_commits().times(1).returning(|_, _| {
            Ok(vec![create_test_forge_commit(
                "abc123",
                "feat: new feature",
                1000,
            )])
        });

        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge.expect_repo_name().return_const("test-repo");

        mock_file_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_release_branch()
            .times(1)
            .returning(|_| {
                Ok(Commit {
                    sha: "new-sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 100,
                    sha: "existing-pr-sha".to_string(),
                }))
            });

        mock_forge.expect_update_pr().times(1).returning(|req| {
            assert_eq!(req.pr_number, 100);
            Ok(())
        });

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|req| {
                assert_eq!(req.pr_number, 100);
                Ok(())
            });

        let result =
            execute(Box::new(mock_forge), Box::new(mock_file_loader)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_no_changes() {
        let mut mock_forge = MockForge::new();
        let mock_file_loader = MockFileLoader::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge
            .expect_get_commits()
            .times(1)
            .returning(|_, _| Ok(vec![]));

        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge.expect_repo_name().return_const("test-repo");

        let result =
            execute(Box::new(mock_forge), Box::new(mock_file_loader)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_non_conventional_commits() {
        let mut mock_forge = MockForge::new();
        let mut mock_file_loader = MockFileLoader::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| Ok(None));

        // Non-conventional commits on first release still create a release
        mock_forge.expect_get_commits().times(1).returning(|_, _| {
            Ok(vec![
                create_test_forge_commit("abc123", "update readme", 1000),
                create_test_forge_commit("def456", "merge branch", 2000),
            ])
        });

        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge.expect_repo_name().return_const("test-repo");

        mock_file_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        // First release includes all commits, so PR is created
        mock_forge
            .expect_create_release_branch()
            .times(1)
            .returning(|_| {
                Ok(Commit {
                    sha: "new-sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_create_pr().times(1).returning(|_| {
            Ok(PullRequest {
                number: 50,
                sha: "pr-sha".to_string(),
            })
        });

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|_| Ok(()));

        let result =
            execute(Box::new(mock_forge), Box::new(mock_file_loader)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_custom_default_branch() {
        let mut mock_forge = MockForge::new();
        let mut mock_file_loader = MockFileLoader::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_get_commits().times(1).returning(|_, _| {
            Ok(vec![create_test_forge_commit(
                "abc123",
                "feat: new feature",
                1000,
            )])
        });

        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("develop".to_string()));

        mock_forge.expect_repo_name().return_const("test-repo");

        mock_file_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_release_branch()
            .times(1)
            .returning(|req| {
                assert_eq!(req.branch, "releasaurus-release-develop");
                assert!(req.message.contains("chore(develop): release"));
                Ok(Commit {
                    sha: "new-sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .returning(|req| {
                assert_eq!(req.head_branch, "releasaurus-release-develop");
                assert_eq!(req.base_branch, "develop");
                Ok(None)
            });

        mock_forge.expect_create_pr().times(1).returning(|req| {
            assert!(req.title.contains("chore(develop): release"));
            assert_eq!(req.base_branch, "develop");
            Ok(PullRequest {
                number: 45,
                sha: "pr-sha".to_string(),
            })
        });

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|_| Ok(()));

        let result =
            execute(Box::new(mock_forge), Box::new(mock_file_loader)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_single_package_includes_version_in_title() {
        let mut mock_forge = MockForge::new();
        let mut mock_file_loader = MockFileLoader::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_get_commits().times(1).returning(|_, _| {
            Ok(vec![create_test_forge_commit(
                "abc123",
                "feat: new feature",
                1000,
            )])
        });

        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge.expect_repo_name().return_const("test-repo");

        mock_file_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_release_branch()
            .times(1)
            .returning(|_| {
                Ok(Commit {
                    sha: "new-sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_create_pr().times(1).returning(|req| {
            assert!(req.title.contains("v0.1.0"));
            assert_eq!(req.title, "chore(main): release v0.1.0");
            Ok(PullRequest {
                number: 46,
                sha: "pr-sha".to_string(),
            })
        });

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|_| Ok(()));

        let result =
            execute(Box::new(mock_forge), Box::new(mock_file_loader)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_handles_config_load_error() {
        let mut mock_forge = MockForge::new();
        let mock_file_loader = MockFileLoader::new();

        let remote_config = create_test_remote_config();

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(|| Err(color_eyre::eyre::eyre!("Config not found")));

        let result =
            execute(Box::new(mock_forge), Box::new(mock_file_loader)).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_with_breaking_changes() {
        let mut mock_forge = MockForge::new();
        let mut mock_file_loader = MockFileLoader::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();
        let existing_tag = create_test_tag("v1.2.3", "1.2.3", "old-sha");

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        let tag_clone = existing_tag.clone();
        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(move |_| Ok(Some(tag_clone.clone())));

        mock_forge.expect_get_commits().times(1).returning(|_, _| {
            Ok(vec![create_test_forge_commit(
                "abc123",
                "feat!: breaking change",
                1000,
            )])
        });

        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge.expect_repo_name().return_const("test-repo");

        mock_file_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_release_branch()
            .times(1)
            .returning(|_| {
                Ok(Commit {
                    sha: "new-sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_create_pr().times(1).returning(|req| {
            assert!(req.title.contains("v2.0.0"));
            Ok(PullRequest {
                number: 47,
                sha: "pr-sha".to_string(),
            })
        });

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|_| Ok(()));

        let result =
            execute(Box::new(mock_forge), Box::new(mock_file_loader)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_mixed_packages_some_without_changes() {
        let mut mock_forge = MockForge::new();
        let mut mock_file_loader = MockFileLoader::new();

        let config = create_test_config(vec![
            create_test_package_config(
                "packages/frontend",
                Some(ReleaseType::Node),
                Some("frontend-v".to_string()),
            ),
            create_test_package_config(
                "packages/backend",
                Some(ReleaseType::Rust),
                Some("backend-v".to_string()),
            ),
        ]);

        let remote_config = create_test_remote_config();

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .with(mockall::predicate::eq("frontend-v"))
            .times(1)
            .returning(|_| Ok(None));

        mock_forge
            .expect_get_commits()
            .with(
                mockall::predicate::eq("packages/frontend"),
                mockall::predicate::eq(None),
            )
            .times(1)
            .returning(|_, _| {
                Ok(vec![create_test_forge_commit(
                    "abc123",
                    "feat: frontend feature",
                    1000,
                )])
            });

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .with(mockall::predicate::eq("backend-v"))
            .times(1)
            .returning(|_| Ok(None));

        mock_forge
            .expect_get_commits()
            .with(
                mockall::predicate::eq("packages/backend"),
                mockall::predicate::eq(None),
            )
            .times(1)
            .returning(|_, _| Ok(vec![]));

        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge.expect_repo_name().return_const("test-repo");

        mock_file_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_release_branch()
            .times(1)
            .returning(|_| {
                Ok(Commit {
                    sha: "new-sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_create_pr().times(1).returning(|_| {
            Ok(PullRequest {
                number: 48,
                sha: "pr-sha".to_string(),
            })
        });

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|_| Ok(()));

        let result =
            execute(Box::new(mock_forge), Box::new(mock_file_loader)).await;

        assert!(result.is_ok());
    }
}
