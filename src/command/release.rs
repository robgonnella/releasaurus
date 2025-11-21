//! Final release publication and tagging command implementation.
use log::*;

use crate::{
    analyzer::Analyzer,
    command::common,
    config::{Config, PackageConfig},
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, RemoteConfig, TAGGED_LABEL},
        request::{ForgeCommit, GetPrRequest, PrLabelsRequest, PullRequest},
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

    let commits = common::get_commits_for_all_packages(
        forge.as_ref(),
        &config.packages,
        &repo_name,
    )
    .await?;

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
            &commits,
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
    commits: &[ForgeCommit],
) -> Result<()> {
    let default_branch = forge.default_branch().await?;
    let remote_config = forge.remote_config();

    let req = GetPrRequest {
        base_branch: default_branch.clone(),
        head_branch: release_branch.to_string(),
    };

    if let Some(merged_pr) = forge.get_merged_release_pr(req).await? {
        create_package_release(
            config,
            &remote_config,
            forge,
            &merged_pr,
            package,
            prerelease_override,
            commits,
        )
        .await?;

        let req = PrLabelsRequest {
            pr_number: merged_pr.number,
            labels: vec![TAGGED_LABEL.into()],
        };

        forge.replace_pr_labels(req).await?;
    } else {
        warn!(
            "releases are up-to-date for package {} and branch {release_branch}: nothing to release",
            package.name,
        );
    }

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
    commits: &[ForgeCommit],
) -> Result<()> {
    let default_branch = forge.default_branch().await?;
    let repo_name = forge.repo_name();
    let tag_prefix = common::get_tag_prefix(package, &repo_name);
    let current_tag = forge.get_latest_tag_for_prefix(&tag_prefix).await?;

    let package_commits = common::filter_commits_for_package(package, commits);

    // Determine prerelease with priority: CLI override > package config > global config
    let prerelease =
        common::get_prerelease(config, package, prerelease_override);

    let analyzer_config = common::generate_analyzer_config(
        config,
        remote_config,
        &default_branch,
        package,
        tag_prefix,
        prerelease,
    );

    let analyzer = Analyzer::new(analyzer_config)?;
    let release = analyzer.analyze(package_commits, current_tag)?;

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
    use crate::{forge::traits::MockForge, test_helpers::*};

    #[tokio::test]
    async fn test_generate_branch_release_with_merged_pr() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_remote_config()
            .returning(create_test_remote_config);

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge.expect_get_merged_release_pr().returning(|_| {
            Ok(Some(create_test_pull_request(123, "merged-sha")))
        });

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(create_test_tag("v1.0.0", "1.0.0", "old-sha")))
            });

        mock_forge
            .expect_tag_commit()
            .withf(|tag_name, sha| tag_name == "v1.1.0" && sha == "merged-sha")
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag_name, _, _| tag_name == "v1.1.0")
            .returning(|_, _, _| Ok(()));

        mock_forge.expect_replace_pr_labels().returning(|req| {
            assert_eq!(req.pr_number, 123);
            assert_eq!(req.labels, vec![TAGGED_LABEL]);
            Ok(())
        });

        let config = create_test_config_simple(vec![(
            "test-repo",
            ".",
            crate::config::ReleaseType::Node,
        )]);

        let package = &config.packages[0];
        let mut commits = vec![create_test_forge_commit(
            "abc123",
            "feat: new feature",
            1000,
        )];
        commits[0].files = vec!["src/main.rs".to_string()];

        let result = generate_branch_release(
            &mock_forge,
            package,
            "releasaurus-release-main",
            &config,
            None,
            &commits,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_generate_branch_release_without_merged_pr() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_remote_config()
            .returning(create_test_remote_config);

        mock_forge
            .expect_get_merged_release_pr()
            .returning(|_| Ok(None));

        let config = create_test_config_simple(vec![(
            "test-repo",
            ".",
            crate::config::ReleaseType::Node,
        )]);

        let package = &config.packages[0];
        let commits = vec![];

        let result = generate_branch_release(
            &mock_forge,
            package,
            "releasaurus-release-main",
            &config,
            None,
            &commits,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_tags_and_publishes() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(create_test_tag("v1.0.0", "1.0.0", "old-sha")))
            });

        mock_forge
            .expect_tag_commit()
            .withf(|tag_name, sha| tag_name == "v1.1.0" && sha == "pr-sha")
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag_name, sha, notes| {
                tag_name == "v1.1.0" && sha == "abc123" && !notes.is_empty()
            })
            .returning(|_, _, _| Ok(()));

        let config = create_test_config_simple(vec![(
            "test-repo",
            ".",
            crate::config::ReleaseType::Node,
        )]);

        let remote_config = create_test_remote_config();
        let merged_pr = create_test_pull_request(456, "pr-sha");
        let package = &config.packages[0];

        let mut commits = vec![create_test_forge_commit(
            "abc123",
            "feat: new feature",
            1000,
        )];
        commits[0].files = vec!["src/main.rs".to_string()];

        let result = create_package_release(
            &config,
            &remote_config,
            &mock_forge,
            &merged_pr,
            package,
            None,
            &commits,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_with_prerelease() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(create_test_tag("v1.0.0", "1.0.0", "old-sha")))
            });

        mock_forge
            .expect_tag_commit()
            .withf(|tag_name, _| tag_name.contains("beta"))
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag_name, _, _| tag_name.contains("beta"))
            .returning(|_, _, _| Ok(()));

        let config = create_test_config_simple(vec![(
            "test-repo",
            ".",
            crate::config::ReleaseType::Node,
        )]);

        let remote_config = create_test_remote_config();
        let merged_pr = create_test_pull_request(789, "pr-sha");
        let package = &config.packages[0];

        let mut commits = vec![create_test_forge_commit(
            "abc123",
            "feat: new feature",
            1000,
        )];
        commits[0].files = vec!["src/main.rs".to_string()];

        let result = create_package_release(
            &config,
            &remote_config,
            &mock_forge,
            &merged_pr,
            package,
            Some("beta".to_string()),
            &commits,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_no_commits() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(create_test_tag("v1.0.0", "1.0.0", "current-sha")))
            });

        let config = create_test_config_simple(vec![(
            "test-repo",
            ".",
            crate::config::ReleaseType::Node,
        )]);

        let remote_config = create_test_remote_config();
        let merged_pr = create_test_pull_request(999, "pr-sha");
        let package = &config.packages[0];
        let commits = vec![];

        let result = create_package_release(
            &config,
            &remote_config,
            &mock_forge,
            &merged_pr,
            package,
            None,
            &commits,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_single_package() {
        let mut mock_forge = MockForge::new();
        mock_forge.expect_load_config().returning(|| {
            Ok(create_test_config_simple(vec![(
                "test-repo",
                ".",
                crate::config::ReleaseType::Node,
            )]))
        });

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_remote_config()
            .returning(create_test_remote_config);

        let mut commits = vec![create_test_forge_commit(
            "abc123",
            "feat: new feature",
            1000,
        )];
        commits[0].files = vec!["src/main.rs".to_string()];
        mock_forge
            .expect_get_commits()
            .returning(move |_| Ok(commits.clone()));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(create_test_tag("v1.0.0", "1.0.0", "old-sha")))
            });

        mock_forge.expect_get_merged_release_pr().returning(|_| {
            Ok(Some(create_test_pull_request(100, "merged-sha")))
        });

        mock_forge.expect_tag_commit().returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .returning(|_, _, _| Ok(()));

        mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

        let result = execute(Box::new(mock_forge), None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_no_merged_pr() {
        let mut mock_forge = MockForge::new();
        mock_forge.expect_load_config().returning(|| {
            Ok(create_test_config_simple(vec![(
                "test-repo",
                ".",
                crate::config::ReleaseType::Node,
            )]))
        });

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_remote_config()
            .returning(create_test_remote_config);

        mock_forge.expect_get_commits().returning(|_| Ok(vec![]));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(create_test_tag("v1.0.0", "1.0.0", "current-sha")))
            });

        mock_forge
            .expect_get_merged_release_pr()
            .returning(|_| Ok(None));

        let result = execute(Box::new(mock_forge), None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_separate_pull_requests() {
        let mut mock_forge = MockForge::new();
        mock_forge.expect_load_config().returning(|| {
            let mut config = create_test_config_simple(vec![
                ("pkg-a", "packages/a", crate::config::ReleaseType::Node),
                ("pkg-b", "packages/b", crate::config::ReleaseType::Node),
            ]);
            config.separate_pull_requests = true;
            Ok(config)
        });

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_remote_config()
            .returning(create_test_remote_config);

        let mut commits_a = vec![create_test_forge_commit(
            "abc123",
            "feat: feature in a",
            1000,
        )];
        commits_a[0].files = vec!["packages/a/index.js".to_string()];

        let mut commits_b = vec![create_test_forge_commit(
            "def456",
            "feat: feature in b",
            2000,
        )];
        commits_b[0].files = vec!["packages/b/index.js".to_string()];

        let mut all_commits = commits_a.clone();
        all_commits.extend(commits_b.clone());

        mock_forge
            .expect_get_commits()
            .returning(move |_| Ok(all_commits.clone()));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(create_test_tag("pkg-a-v1.0.0", "1.0.0", "old-sha")))
            });

        mock_forge.expect_get_merged_release_pr().returning(|req| {
            if req.head_branch.contains("pkg-a") {
                Ok(Some(create_test_pull_request(200, "merged-sha-a")))
            } else if req.head_branch.contains("pkg-b") {
                Ok(Some(create_test_pull_request(201, "merged-sha-b")))
            } else {
                Ok(None)
            }
        });

        mock_forge.expect_tag_commit().returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .returning(|_, _, _| Ok(()));

        mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

        let result = execute(Box::new(mock_forge), None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_prerelease_override() {
        let mut mock_forge = MockForge::new();
        mock_forge.expect_load_config().returning(|| {
            Ok(create_test_config_simple(vec![(
                "test-repo",
                ".",
                crate::config::ReleaseType::Node,
            )]))
        });

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_remote_config()
            .returning(create_test_remote_config);

        let mut commits = vec![create_test_forge_commit(
            "abc123",
            "feat: new feature",
            1000,
        )];
        commits[0].files = vec!["src/main.rs".to_string()];
        mock_forge
            .expect_get_commits()
            .returning(move |_| Ok(commits.clone()));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(create_test_tag("v1.0.0", "1.0.0", "old-sha")))
            });

        mock_forge.expect_get_merged_release_pr().returning(|_| {
            Ok(Some(create_test_pull_request(300, "merged-sha")))
        });

        mock_forge
            .expect_tag_commit()
            .withf(|tag_name, _| tag_name.contains("rc"))
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag_name, _, _| tag_name.contains("rc"))
            .returning(|_, _, _| Ok(()));

        mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

        let result =
            execute(Box::new(mock_forge), Some("rc".to_string())).await;
        assert!(result.is_ok());
    }
}
