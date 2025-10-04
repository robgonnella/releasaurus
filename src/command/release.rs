//! Final release publication and tagging command implementation.
use log::*;

use crate::{
    analyzer::Analyzer,
    command::common,
    config,
    forge::{
        config::{RemoteConfig, TAGGED_LABEL},
        request::{PrLabelsRequest, PullRequest},
        traits::Forge,
    },
    result::Result,
};

/// Execute release command by finding the merged release PR, tagging commits,
/// and publishing releases to the forge platform.
pub async fn execute(forge: Box<dyn Forge>) -> Result<()> {
    let remote_config = forge.remote_config();
    let merged_pr = forge.get_merged_release_pr().await?;

    if merged_pr.is_none() {
        warn!("releases are up-to-date: nothing to release");
        return Ok(());
    }

    let merged_pr = merged_pr.unwrap();

    let config = forge.load_config().await?;

    process_packages_for_release(
        forge.as_ref(),
        &remote_config,
        &merged_pr,
        &config,
    )
    .await?;

    let req = PrLabelsRequest {
        pr_number: merged_pr.number,
        labels: vec![TAGGED_LABEL.into()],
    };
    forge.replace_pr_labels(req).await?;

    Ok(())
}

/// Iterate through all configured packages and create releases for each one.
async fn process_packages_for_release(
    forge: &dyn Forge,
    remote_config: &RemoteConfig,
    merged_pr: &PullRequest,
    conf: &config::Config,
) -> Result<()> {
    for package in &conf.packages {
        create_package_release(conf, remote_config, forge, merged_pr, package)
            .await?
    }

    Ok(())
}

/// Analyze commits since last tag, determine next version, create git tag, and
/// publish release with generated notes.
async fn create_package_release(
    config: &config::Config,
    remote_config: &RemoteConfig,
    forge: &dyn Forge,
    merged_pr: &PullRequest,
    package: &config::PackageConfig,
) -> Result<()> {
    let tag_prefix = common::get_tag_prefix(package);
    let current_tag = forge.get_latest_tag_for_prefix(&tag_prefix).await?;
    let current_sha = current_tag.clone().map(|t| t.sha);
    let commits = forge.get_commits(&package.path, current_sha).await?;

    let analyzer_config =
        common::generate_analyzer_config(config, remote_config, tag_prefix);

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
        config::ReleaseType, forge::traits::MockForge, test_helpers::*,
    };

    #[tokio::test]
    async fn test_execute_with_no_merged_pr() {
        let mut mock_forge = MockForge::new();

        let remote_config = create_test_remote_config();

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(|| Ok(None));

        let result = execute(Box::new(mock_forge)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_single_package_no_existing_tag() {
        let mut mock_forge = MockForge::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();
        let merged_pr = create_test_pull_request(100, "merge-sha");

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(move || Ok(Some(merged_pr.clone())));

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
            .expect_tag_commit()
            .with(
                mockall::predicate::eq("v0.1.0"),
                mockall::predicate::eq("merge-sha"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag, sha, _notes| tag == "v0.1.0" && sha == "abc123")
            .times(1)
            .returning(|_, _, _| Ok(()));

        mock_forge
            .expect_replace_pr_labels()
            .withf(|req| {
                req.pr_number == 100
                    && req.labels.contains(&TAGGED_LABEL.into())
            })
            .times(1)
            .returning(|_| Ok(()));

        let result = execute(Box::new(mock_forge)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_single_package_existing_tag() {
        let mut mock_forge = MockForge::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();
        let merged_pr = create_test_pull_request(101, "merge-sha-2");
        let existing_tag = create_test_tag("v1.0.0", "1.0.0", "old-sha");

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(move || Ok(Some(merged_pr.clone())));

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
                "def456",
                "fix: fix critical bug",
                2000,
            )])
        });

        mock_forge
            .expect_tag_commit()
            .with(
                mockall::predicate::eq("v1.0.1"),
                mockall::predicate::eq("merge-sha-2"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag, sha, _notes| tag == "v1.0.1" && sha == "def456")
            .times(1)
            .returning(|_, _, _| Ok(()));

        mock_forge
            .expect_replace_pr_labels()
            .withf(|req| req.pr_number == 101)
            .times(1)
            .returning(|_| Ok(()));

        let result = execute(Box::new(mock_forge)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_multiple_packages() {
        let mut mock_forge = MockForge::new();

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
        let merged_pr = create_test_pull_request(102, "multi-merge-sha");

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(move || Ok(Some(merged_pr.clone())));

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        // First package
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
                    "abc111",
                    "feat: frontend feature",
                    1000,
                )])
            });

        mock_forge
            .expect_tag_commit()
            .with(
                mockall::predicate::eq("frontend-v0.1.0"),
                mockall::predicate::eq("multi-merge-sha"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag, sha, _notes| {
                tag == "frontend-v0.1.0" && sha == "abc111"
            })
            .times(1)
            .returning(|_, _, _| Ok(()));

        // Second package
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
                    "def222",
                    "feat: backend feature",
                    2000,
                )])
            });

        mock_forge
            .expect_tag_commit()
            .with(
                mockall::predicate::eq("backend-v0.1.0"),
                mockall::predicate::eq("multi-merge-sha"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag, sha, _notes| {
                tag == "backend-v0.1.0" && sha == "def222"
            })
            .times(1)
            .returning(|_, _, _| Ok(()));

        mock_forge
            .expect_replace_pr_labels()
            .withf(|req| req.pr_number == 102)
            .times(1)
            .returning(|_| Ok(()));

        let result = execute(Box::new(mock_forge)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_package_no_changes() {
        let mut mock_forge = MockForge::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();
        let merged_pr = create_test_pull_request(103, "no-change-sha");

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(move || Ok(Some(merged_pr.clone())));

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| Ok(None));

        // No commits - no release should be created
        mock_forge
            .expect_get_commits()
            .times(1)
            .returning(|_, _| Ok(vec![]));

        // Should not call tag_commit or create_release
        mock_forge.expect_tag_commit().times(0);
        mock_forge.expect_create_release().times(0);

        // Should still update labels
        mock_forge
            .expect_replace_pr_labels()
            .withf(|req| req.pr_number == 103)
            .times(1)
            .returning(|_| Ok(()));

        let result = execute(Box::new(mock_forge)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_breaking_changes() {
        let mut mock_forge = MockForge::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();
        let merged_pr = create_test_pull_request(104, "breaking-sha");
        let existing_tag = create_test_tag("v1.2.3", "1.2.3", "old-sha");

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(move || Ok(Some(merged_pr.clone())));

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
                "break123",
                "feat!: breaking API change",
                3000,
            )])
        });

        // Breaking change should bump major version: 1.2.3 -> 2.0.0
        mock_forge
            .expect_tag_commit()
            .with(
                mockall::predicate::eq("v2.0.0"),
                mockall::predicate::eq("breaking-sha"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag, sha, _notes| tag == "v2.0.0" && sha == "break123")
            .times(1)
            .returning(|_, _, _| Ok(()));

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|_| Ok(()));

        let result = execute(Box::new(mock_forge)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_handles_config_load_error() {
        let mut mock_forge = MockForge::new();

        let remote_config = create_test_remote_config();
        let merged_pr = create_test_pull_request(105, "error-sha");

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(move || Ok(Some(merged_pr.clone())));

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(|| Err(color_eyre::eyre::eyre!("Config not found")));

        let result = execute(Box::new(mock_forge)).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_handles_tag_commit_error() {
        let mut mock_forge = MockForge::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();
        let merged_pr = create_test_pull_request(106, "tag-error-sha");

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(move || Ok(Some(merged_pr.clone())));

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
                "abc789",
                "feat: new feature",
                1000,
            )])
        });

        mock_forge.expect_tag_commit().times(1).returning(|_, _| {
            Err(color_eyre::eyre::eyre!("Failed to create tag"))
        });

        let result = execute(Box::new(mock_forge)).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_handles_create_release_error() {
        let mut mock_forge = MockForge::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();
        let merged_pr = create_test_pull_request(107, "release-error-sha");

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(move || Ok(Some(merged_pr.clone())));

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
                "xyz999",
                "feat: new feature",
                1000,
            )])
        });

        mock_forge
            .expect_tag_commit()
            .times(1)
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .times(1)
            .returning(|_, _, _| {
                Err(color_eyre::eyre::eyre!("Failed to create release"))
            });

        let result = execute(Box::new(mock_forge)).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_with_custom_tag_prefix() {
        let mut mock_forge = MockForge::new();

        let config = create_test_config(vec![create_test_package_config(
            "packages/api",
            Some(ReleaseType::Node),
            Some("api-v".to_string()),
        )]);

        let remote_config = create_test_remote_config();
        let merged_pr = create_test_pull_request(108, "custom-prefix-sha");

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(move || Ok(Some(merged_pr.clone())));

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .with(mockall::predicate::eq("api-v"))
            .times(1)
            .returning(|_| Ok(None));

        mock_forge
            .expect_get_commits()
            .with(
                mockall::predicate::eq("packages/api"),
                mockall::predicate::eq(None),
            )
            .times(1)
            .returning(|_, _| {
                Ok(vec![create_test_forge_commit(
                    "custom123",
                    "feat: api feature",
                    1000,
                )])
            });

        mock_forge
            .expect_tag_commit()
            .with(
                mockall::predicate::eq("api-v0.1.0"),
                mockall::predicate::eq("custom-prefix-sha"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag, sha, _notes| tag == "api-v0.1.0" && sha == "custom123")
            .times(1)
            .returning(|_, _, _| Ok(()));

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|_| Ok(()));

        let result = execute(Box::new(mock_forge)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_mixed_packages_some_without_changes() {
        let mut mock_forge = MockForge::new();

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
        let merged_pr = create_test_pull_request(109, "mixed-sha");

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(move || Ok(Some(merged_pr.clone())));

        mock_forge
            .expect_load_config()
            .times(1)
            .returning(move || Ok(config.clone()));

        // First package has changes
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
                    "front123",
                    "feat: frontend update",
                    1000,
                )])
            });

        mock_forge
            .expect_tag_commit()
            .with(
                mockall::predicate::eq("frontend-v0.1.0"),
                mockall::predicate::eq("mixed-sha"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag, sha, _notes| {
                tag == "frontend-v0.1.0" && sha == "front123"
            })
            .times(1)
            .returning(|_, _, _| Ok(()));

        // Second package has no changes
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

        // No tag or release for second package

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|_| Ok(()));

        let result = execute(Box::new(mock_forge)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_replaces_labels_with_tagged() {
        let mut mock_forge = MockForge::new();

        let config = create_test_config(vec![create_test_package_config(
            ".",
            Some(ReleaseType::Node),
            Some("v".to_string()),
        )]);

        let remote_config = create_test_remote_config();
        let merged_pr = create_test_pull_request(110, "label-sha");

        mock_forge
            .expect_remote_config()
            .return_const(remote_config.clone());

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(move || Ok(Some(merged_pr.clone())));

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
                "label123",
                "feat: test",
                1000,
            )])
        });

        mock_forge
            .expect_tag_commit()
            .times(1)
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|_tag, sha, _notes| sha == "label123")
            .times(1)
            .returning(|_, _, _| Ok(()));

        // Verify the label is set correctly
        mock_forge
            .expect_replace_pr_labels()
            .withf(|req| {
                req.pr_number == 110
                    && req.labels.len() == 1
                    && req.labels[0] == TAGGED_LABEL
            })
            .times(1)
            .returning(|_| Ok(()));

        let result = execute(Box::new(mock_forge)).await;

        assert!(result.is_ok());
    }
}
