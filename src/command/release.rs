//! Final release publication and tagging command implementation.
use color_eyre::eyre::OptionExt;
use regex::Regex;
use serde::Deserialize;
use std::{path::Path, sync::LazyLock};

use crate::{
    command::common,
    config::PackageConfig,
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, TAGGED_LABEL},
        request::{GetPrRequest, PrLabelsRequest, PullRequest},
        traits::Forge,
    },
    result::Result,
};

static METADATA_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?ms)<!--(?<metadata>.*)-->"#).unwrap());

#[derive(Debug, Deserialize)]
struct Metadata {
    pub tag: String,
    pub sha: String,
}

#[derive(Debug, Deserialize)]
struct MetadataJson {
    pub metadata: Metadata,
}

/// Execute release command by finding the merged release PR, tagging commits,
/// and publishing releases to the forge platform.
pub async fn execute(forge: Box<dyn Forge>) -> Result<()> {
    let repo_name = forge.repo_name();
    let mut config = forge.load_config().await?;
    let config = common::process_config(&repo_name, &mut config);
    let default_branch = forge.default_branch();

    for package in config.packages.iter() {
        let mut release_branch =
            format!("{DEFAULT_PR_BRANCH_PREFIX}-{default_branch}");

        if config.separate_pull_requests {
            release_branch = format!(
                "{DEFAULT_PR_BRANCH_PREFIX}-{default_branch}-{}",
                package.name
            );
        }

        let default_branch = forge.default_branch();

        let req = GetPrRequest {
            base_branch: default_branch.clone(),
            head_branch: release_branch.to_string(),
        };

        if let Some(merged_pr) = forge.get_merged_release_pr(req).await? {
            create_package_release(
                forge.as_ref(),
                package,
                &merged_pr,
                &config.changelog.release_start_regex,
            )
            .await?;

            let req = PrLabelsRequest {
                pr_number: merged_pr.number,
                labels: vec![TAGGED_LABEL.into()],
            };

            forge.replace_pr_labels(req).await?;
        }
    }

    Ok(())
}

/// Creates release for a targeted package and merged PR
async fn create_package_release(
    forge: &dyn Forge,
    package: &PackageConfig,
    merged_pr: &PullRequest,
    release_start_regex: &str,
) -> Result<()> {
    let meta_caps = METADATA_REGEX
        .captures(&merged_pr.body)
        .ok_or_eyre("failed to detect release metadata in merged PR")?;

    let metadata_str = meta_caps
        .name("metadata")
        .ok_or_eyre("failed to parse metadata from PR body")?
        .as_str();

    let json: MetadataJson = serde_json::from_str(metadata_str)?;
    let metadata = json.metadata;

    let changelog_path = Path::new(&package.workspace_root)
        .join(&package.path)
        .join("CHANGELOG.md")
        .display()
        .to_string()
        .replace("./", "");

    let changelog_content = forge
        .get_file_content(&changelog_path)
        .await?
        .ok_or_eyre("failed to find CHANGELOG.md for package")?;

    // The logic below handles 2 cases, single release in changelog, and
    // multiple releases in changelog. When there is a single release in the
    // changelog we using the following regex to capture the notes
    // ({release_start_regex}.*)
    // When there are 2 or more releases in the changelog, we use the following
    // regex to capture only the latest release's notes
    // ({release_start_regex}.*)\n*{release_start_regex}

    // First we count the occurrences of the release_start_regex matcher
    // to find out which case we need to account for
    let start_regex =
        Regex::new(&format!(r#"(?ms)(?<start>{release_start_regex})"#))?;

    let release_count = start_regex.find_iter(&changelog_content).count();

    // Set notes_regex to match case for multiple releases in changelog
    let mut notes_regex = Regex::new(&format!(
        r#"(?ms)(?<notes>{release_start_regex}.*)\n*{release_start_regex}"#,
    ))?;

    // update regex if we only have 1 release in changelog
    if release_count == 1 {
        notes_regex =
            Regex::new(&format!(r#"(?ms)(?<notes>{release_start_regex}.*)"#,))?;
    }

    let caps = notes_regex
        .captures(&changelog_content)
        .ok_or_eyre("failed to capture release notes from CHANGELOG.md")?;

    let notes = caps
        .name("notes")
        .ok_or_eyre("failed to parse notes from CHANGELOG.md")?
        .as_str();

    forge.tag_commit(&metadata.tag, &merged_pr.sha).await?;

    forge
        .create_release(&metadata.tag, &metadata.sha, notes.trim())
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ReleaseType, forge::traits::MockForge, test_helpers::*,
    };

    fn create_pr_body_with_metadata(tag: &str, sha: &str) -> String {
        format!(
            r#"Release PR body

<!--{{"metadata":{{"tag":"{}","sha":"{}"}}}}-->
"#,
            tag, sha
        )
    }

    #[tokio::test]
    async fn test_execute_single_package_with_merged_pr() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_default_branch()
            .times(2)
            .returning(|| "main".to_string());

        let config = create_test_config_simple(vec![(
            "my-package",
            ".",
            ReleaseType::Node,
        )]);

        mock_forge
            .expect_load_config()
            .returning(move || Ok(config.clone()));

        let pr_body = create_pr_body_with_metadata("v1.0.0", "abc123");

        mock_forge
            .expect_get_merged_release_pr()
            .returning(move |_| {
                Ok(Some(PullRequest {
                    number: 42,
                    sha: "merge-sha".to_string(),
                    body: pr_body.clone(),
                }))
            });

        mock_forge.expect_get_file_content().returning(|path| {
            if path.contains("CHANGELOG.md") {
                Ok(Some("# [1.0.0]\n\n## Features\n- New feature".to_string()))
            } else {
                Ok(None)
            }
        });

        mock_forge
            .expect_tag_commit()
            .withf(|tag, sha| tag == "v1.0.0" && sha == "merge-sha")
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag, sha, notes| {
                tag == "v1.0.0" && sha == "abc123" && notes.contains("1.0.0")
            })
            .returning(|_, _, _| Ok(()));

        mock_forge
            .expect_replace_pr_labels()
            .withf(|req| {
                req.pr_number == 42
                    && req.labels.contains(&TAGGED_LABEL.to_string())
            })
            .returning(|_| Ok(()));

        let result = execute(Box::new(mock_forge)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_multiple_packages() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_default_branch()
            .times(3)
            .returning(|| "main".to_string());

        let config = create_test_config_simple(vec![
            ("pkg-a", "packages/a", ReleaseType::Node),
            ("pkg-b", "packages/b", ReleaseType::Rust),
        ]);
        mock_forge
            .expect_load_config()
            .returning(move || Ok(config.clone()));

        let pr_body_a = create_pr_body_with_metadata("pkg-a-v1.0.0", "sha-a");
        let pr_body_b = create_pr_body_with_metadata("pkg-b-v2.0.0", "sha-b");

        mock_forge
            .expect_get_merged_release_pr()
            .times(2)
            .returning(move |_| {
                static mut CALL_COUNT: usize = 0;
                unsafe {
                    CALL_COUNT += 1;
                    if CALL_COUNT == 1 {
                        Ok(Some(PullRequest {
                            number: 10,
                            sha: "merge-sha-a".to_string(),
                            body: pr_body_a.clone(),
                        }))
                    } else {
                        Ok(Some(PullRequest {
                            number: 20,
                            sha: "merge-sha-b".to_string(),
                            body: pr_body_b.clone(),
                        }))
                    }
                }
            });

        mock_forge.expect_get_file_content().returning(|path| {
            if path.contains("CHANGELOG.md") {
                Ok(Some("# [1.0.0]\n\nRelease notes".to_string()))
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

        let result = execute(Box::new(mock_forge)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_separate_pull_requests() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_default_branch()
            .times(2)
            .returning(|| "main".to_string());

        let mut config =
            create_test_config_simple(vec![("pkg-a", ".", ReleaseType::Node)]);
        config.separate_pull_requests = true;

        mock_forge
            .expect_load_config()
            .returning(move || Ok(config.clone()));

        let pr_body = create_pr_body_with_metadata("v1.0.0", "sha");

        mock_forge
            .expect_get_merged_release_pr()
            .withf(|req| req.head_branch == "releasaurus-release-main-pkg-a")
            .returning(move |_| {
                Ok(Some(PullRequest {
                    number: 99,
                    sha: "merge-sha".to_string(),
                    body: pr_body.clone(),
                }))
            });

        mock_forge
            .expect_get_file_content()
            .returning(|_| Ok(Some("# [1.0.0]\n\nNotes".to_string())));

        mock_forge.expect_tag_commit().returning(|_, _| Ok(()));
        mock_forge
            .expect_create_release()
            .returning(|_, _, _| Ok(()));
        mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

        let result = execute(Box::new(mock_forge)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_skips_package_without_merged_pr() {
        let mut mock_forge = MockForge::new();

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_default_branch()
            .times(2)
            .returning(|| "main".to_string());

        let config =
            create_test_config_simple(vec![("pkg", ".", ReleaseType::Node)]);
        mock_forge
            .expect_load_config()
            .returning(move || Ok(config.clone()));

        mock_forge
            .expect_get_merged_release_pr()
            .returning(|_| Ok(None));

        // Should not call any release-related methods
        let result = execute(Box::new(mock_forge)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_single_release_in_changelog() {
        let mut mock_forge = MockForge::new();

        let package = create_test_config_simple(vec![(
            "my-package",
            ".",
            ReleaseType::Node,
        )])
        .packages[0]
            .clone();

        let pr_body = create_pr_body_with_metadata("v1.0.0", "sha123");
        let merged_pr = PullRequest {
            number: 1,
            sha: "merge-sha".to_string(),
            body: pr_body,
        };

        let changelog =
            "# [1.0.0] - 2024-01-01\n\n## Features\n- Added feature";
        mock_forge
            .expect_get_file_content()
            .returning(move |_| Ok(Some(changelog.to_string())));

        mock_forge
            .expect_tag_commit()
            .withf(|tag, sha| tag == "v1.0.0" && sha == "merge-sha")
            .returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|tag, sha, notes| {
                tag == "v1.0.0"
                    && sha == "sha123"
                    && notes.contains("[1.0.0]")
                    && notes.contains("Features")
            })
            .returning(|_, _, _| Ok(()));

        let result = create_package_release(
            &mock_forge,
            &package,
            &merged_pr,
            r"^#\s\[",
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_multiple_releases_in_changelog() {
        let mut mock_forge = MockForge::new();

        let package = create_test_config_simple(vec![(
            "my-package",
            ".",
            ReleaseType::Node,
        )])
        .packages[0]
            .clone();

        let pr_body = create_pr_body_with_metadata("v2.0.0", "sha456");
        let merged_pr = PullRequest {
            number: 1,
            sha: "merge-sha".to_string(),
            body: pr_body,
        };

        let changelog = r#"# [2.0.0] - 2024-02-01

## Breaking Changes
- Breaking change

# [1.0.0] - 2024-01-01

## Features
- Initial release"#;

        mock_forge
            .expect_get_file_content()
            .returning(move |_| Ok(Some(changelog.to_string())));

        mock_forge.expect_tag_commit().returning(|_, _| Ok(()));

        mock_forge
            .expect_create_release()
            .withf(|_, _, notes| {
                notes.contains("[2.0.0]")
                    && notes.contains("Breaking")
                    && !notes.contains("[1.0.0]")
            })
            .returning(|_, _, _| Ok(()));

        let result = create_package_release(
            &mock_forge,
            &package,
            &merged_pr,
            r"^#\s\[",
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_nested_package_path() {
        let mut mock_forge = MockForge::new();

        let package = create_test_config_simple(vec![(
            "nested-pkg",
            "packages/nested",
            ReleaseType::Node,
        )])
        .packages[0]
            .clone();

        let pr_body = create_pr_body_with_metadata("v1.0.0", "sha");
        let merged_pr = PullRequest {
            number: 1,
            sha: "merge-sha".to_string(),
            body: pr_body,
        };

        mock_forge
            .expect_get_file_content()
            .withf(|path| path == "packages/nested/CHANGELOG.md")
            .returning(|_| Ok(Some("# [1.0.0]\n\nNotes".to_string())));

        mock_forge.expect_tag_commit().returning(|_, _| Ok(()));
        mock_forge
            .expect_create_release()
            .returning(|_, _, _| Ok(()));

        let result = create_package_release(
            &mock_forge,
            &package,
            &merged_pr,
            r"^#\s\[",
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_package_release_fails_without_metadata() {
        let mock_forge = MockForge::new();

        let package =
            create_test_config_simple(vec![("pkg", ".", ReleaseType::Node)])
                .packages[0]
                .clone();

        let merged_pr = PullRequest {
            number: 1,
            sha: "merge-sha".to_string(),
            body: "PR body without metadata".to_string(),
        };

        let result = create_package_release(
            &mock_forge,
            &package,
            &merged_pr,
            r"^#\s\[",
        )
        .await;

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("metadata"));
    }

    #[tokio::test]
    async fn test_create_package_release_fails_without_changelog() {
        let mut mock_forge = MockForge::new();

        let package =
            create_test_config_simple(vec![("pkg", ".", ReleaseType::Node)])
                .packages[0]
                .clone();

        let pr_body = create_pr_body_with_metadata("v1.0.0", "sha");
        let merged_pr = PullRequest {
            number: 1,
            sha: "merge-sha".to_string(),
            body: pr_body,
        };

        mock_forge.expect_get_file_content().returning(|_| Ok(None));

        let result = create_package_release(
            &mock_forge,
            &package,
            &merged_pr,
            r"^#\s\[",
        )
        .await;

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("CHANGELOG.md"));
    }
}
