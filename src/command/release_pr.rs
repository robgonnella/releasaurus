//! Release pull request creation command implementation.

use color_eyre::eyre::eyre;
use log::*;
use std::{collections::HashMap, path::Path};

use crate::{
    analyzer::Analyzer,
    command::common,
    config::{Config, ReleaseType},
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, PENDING_LABEL},
        request::{
            CreateBranchRequest, CreatePrRequest, FileChange, FileUpdateType,
            GetPrRequest, PrLabelsRequest, UpdatePrRequest,
        },
        traits::Forge,
    },
    result::{PendingReleaseError, ReleasablePackage, Result},
    updater::framework::Framework,
};

#[derive(Debug, Clone)]
struct ReleasePr {
    pub title: String,
    pub body: String,
    pub file_changes: Vec<FileChange>,
}

/// Analyze commits since last tags, generate changelogs, update version files,
/// and create or update release PR.
pub async fn execute(
    forge: Box<dyn Forge>,
    prerelease_override: Option<String>,
) -> Result<()> {
    let mut config = forge.load_config().await?;
    let repo_name = forge.repo_name();
    let config = common::process_config(&repo_name, &mut config);

    let releasable_packages = get_releasable_packages(
        &config,
        forge.as_ref(),
        prerelease_override.clone(),
    )
    .await?;

    info!("releasable packages: {:#?}", releasable_packages);

    let prs_by_branch = gather_release_prs_by_branch(
        &releasable_packages,
        forge.as_ref(),
        &config,
    )
    .await?;

    if prs_by_branch.is_empty() {
        return Ok(());
    }

    create_branch_release_prs(prs_by_branch, forge.as_ref()).await?;

    Ok(())
}

async fn create_branch_release_prs(
    prs_by_branch: HashMap<String, Vec<ReleasePr>>,
    forge: &dyn Forge,
) -> Result<()> {
    let default_branch = forge.default_branch().await?;
    // create a single pr per branch
    for (release_branch, prs) in prs_by_branch {
        let single_pr = prs.len() == 1;

        let mut title = format!("chore({default_branch}): release");
        let mut body: Vec<String> = vec![];
        let mut file_changes: Vec<FileChange> = vec![];

        for pr in prs {
            if single_pr {
                title = pr.title;
            }

            body.push(pr.body);
            file_changes.extend(pr.file_changes);
        }

        let in_process_release_req = GetPrRequest {
            base_branch: default_branch.clone(),
            head_branch: release_branch.clone(),
        };

        let pending_release =
            forge.get_merged_release_pr(in_process_release_req).await?;

        if let Some(pr) = pending_release {
            error!("pending release: {:#?}", pr);
            return Err(PendingReleaseError {
                branch: release_branch.clone(),
                pr_number: pr.number,
            }
            .into());
        }

        info!("creating / updating release branch: {release_branch}");
        forge
            .create_release_branch(CreateBranchRequest {
                branch: release_branch.clone(),
                message: title.clone(),
                file_changes,
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
                    base_branch: default_branch.clone(),
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

async fn gather_release_prs_by_branch(
    releasable_packages: &[ReleasablePackage],
    forge: &dyn Forge,
    config: &Config,
) -> Result<HashMap<String, Vec<ReleasePr>>> {
    let default_branch = forge.default_branch().await?;

    let mut prs_by_branch: HashMap<String, Vec<ReleasePr>> = HashMap::new();

    for pkg in releasable_packages.iter() {
        let mut file_changes =
            Framework::update_package(forge, pkg, releasable_packages).await?;

        let mut title =
            format!("chore({}): release {}", default_branch, pkg.name);

        let mut release_branch =
            format!("{DEFAULT_PR_BRANCH_PREFIX}-{}", default_branch);

        if config.separate_pull_requests {
            release_branch = format!("{release_branch}-{}", pkg.name);
        }

        let mut start_details = "<details>";

        // auto-open dropdown if there's only one package
        // or if separate_pull_requests
        if releasable_packages.len() == 1 || config.separate_pull_requests {
            start_details = "<details open>";
        }

        let tag = pkg.release.tag.clone().ok_or(eyre!(
            "Projected release should have a projected tag but failed to detect one. Please report this issue here: https://github.com/robgonnella/releasaurus/issues"
        ))?;

        title = format!("{title} {}", tag.name);

        // create the drop down
        let body = format!(
            "{start_details}<summary>{}</summary>\n\n{}</details>",
            tag.name, pkg.release.notes
        );

        let changelog_path = Path::new(&pkg.workspace_root)
            .join(&pkg.path)
            .join("CHANGELOG.md")
            .display()
            .to_string()
            .replace("./", "");

        file_changes.push(FileChange {
            content: format!("{}\n\n", pkg.release.notes),
            path: changelog_path,
            update_type: FileUpdateType::Prepend,
        });

        let prs = prs_by_branch.get_mut(&release_branch);

        if let Some(prs) = prs {
            prs.push(ReleasePr {
                title,
                body,
                file_changes: file_changes.clone(),
            })
        } else {
            prs_by_branch.insert(
                release_branch.clone(),
                vec![ReleasePr {
                    title,
                    body,
                    file_changes: file_changes.clone(),
                }],
            );
        };
    }

    Ok(prs_by_branch)
}

async fn get_releasable_packages(
    config: &Config,
    forge: &dyn Forge,
    prerelease_override: Option<String>,
) -> Result<Vec<ReleasablePackage>> {
    let repo_name = forge.repo_name();
    let remote_config = forge.remote_config();

    let mut manifest: Vec<ReleasablePackage> = vec![];

    for package in config.packages.iter() {
        let tag_prefix = common::get_tag_prefix(package, &repo_name);

        info!(
            "processing package: \n\tname: {}, \n\tworkspace_root: {}, \n\tpath: {}, \n\ttag_prefix: {}",
            package.name, package.workspace_root, package.path, tag_prefix
        );

        let current_tag = forge.get_latest_tag_for_prefix(&tag_prefix).await?;

        info!(
            "package_name: {}, current tag {:#?}",
            package.name, current_tag
        );

        let current_sha = current_tag.clone().map(|t| t.sha);

        let package_full_path = Path::new(&package.workspace_root)
            .join(&package.path)
            .display()
            .to_string()
            .replace("./", "");

        let mut package_paths = vec![package_full_path];

        if let Some(additional_paths) = package.additional_paths.clone() {
            package_paths.extend(additional_paths);
        }

        let commits =
            common::get_package_commits(forge, current_sha, &package_paths)
                .await?;

        info!("processing commits for package: {}", package.name);

        let analyzer_config = common::generate_analyzer_config(
            config,
            &remote_config,
            package,
            tag_prefix.clone(),
            prerelease_override.clone(),
        );

        let analyzer = Analyzer::new(analyzer_config)?;

        if let Some(release) = analyzer.analyze(commits, current_tag)? {
            info!("package: {}, release: {:#?}", package.name, release);

            let release_type =
                package.release_type.clone().unwrap_or(ReleaseType::Generic);

            manifest.push(ReleasablePackage {
                name: package.name.clone(),
                path: package.path.clone(),
                workspace_root: package.workspace_root.clone(),
                release_type,
                release,
            });
        } else {
            info!("nothing to release for package: {}", package.name);
        }
    }

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::release::{Release, Tag},
        config::{PackageConfig, ReleaseType},
        forge::{
            request::{ForgeCommit, PullRequest},
            traits::MockForge,
        },
        result::ReleasablePackage,
        test_helpers,
    };
    use semver::Version as SemVer;

    #[tokio::test]
    async fn get_releasable_packages_returns_packages_with_releases() {
        let config = test_helpers::create_test_config(vec![PackageConfig {
            name: "my-package".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: None,
        }]);

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_remote_config()
            .returning(test_helpers::create_test_remote_config);

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_get_commits().times(1).returning(|_| {
            Ok(vec![ForgeCommit {
                id: "abc123".to_string(),
                link: "https://github.com/test/repo/commit/abc123".to_string(),
                author_name: "Test Author".to_string(),
                author_email: "test@example.com".to_string(),
                merge_commit: false,
                message: "feat: new feature".to_string(),
                timestamp: 1000,
                files: vec!["src/main.rs".to_string()],
            }])
        });

        let result = get_releasable_packages(&config, &mock_forge, None)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "my-package");
        assert!(result[0].release.tag.is_some());
    }

    #[tokio::test]
    async fn get_releasable_packages_returns_empty_when_no_changes() {
        let config = test_helpers::create_test_config(vec![PackageConfig {
            name: "my-package".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: None,
        }]);

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_remote_config()
            .returning(test_helpers::create_test_remote_config);

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge
            .expect_get_commits()
            .times(1)
            .returning(|_| Ok(vec![]));

        let result = get_releasable_packages(&config, &mock_forge, None)
            .await
            .unwrap();

        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn get_releasable_packages_applies_prerelease_override() {
        let config = test_helpers::create_test_config(vec![PackageConfig {
            name: "my-package".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: None,
        }]);

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_remote_config()
            .returning(test_helpers::create_test_remote_config);

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_get_commits().times(1).returning(|_| {
            Ok(vec![ForgeCommit {
                id: "abc123".to_string(),
                link: "https://github.com/test/repo/commit/abc123".to_string(),
                author_name: "Test Author".to_string(),
                author_email: "test@example.com".to_string(),
                merge_commit: false,
                message: "feat: new feature".to_string(),
                timestamp: 1000,
                files: vec!["src/main.rs".to_string()],
            }])
        });

        let result = get_releasable_packages(
            &config,
            &mock_forge,
            Some("alpha".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(result.len(), 1);
        let tag = result[0].release.tag.as_ref().unwrap();
        assert!(tag.semver.pre.as_str().contains("alpha"));
    }

    #[tokio::test]
    async fn gather_release_prs_creates_single_branch_for_multiple_packages() {
        let mut config = test_helpers::create_test_config(vec![
            PackageConfig {
                name: "package-a".into(),
                path: "packages/a".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Generic),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            },
            PackageConfig {
                name: "package-b".into(),
                path: "packages/b".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Generic),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            },
        ]);
        config.separate_pull_requests = false;

        let packages = vec![
            ReleasablePackage {
                name: "package-a".to_string(),
                path: "packages/a".to_string(),
                workspace_root: ".".to_string(),
                release_type: ReleaseType::Generic,
                release: Release {
                    tag: Some(Tag {
                        sha: "sha1".to_string(),
                        name: "v1.0.0".to_string(),
                        semver: SemVer::parse("1.0.0").unwrap(),
                    }),
                    link: String::new(),
                    sha: "sha1".to_string(),
                    commits: vec![],
                    include_author: false,
                    notes: "Release notes A".to_string(),
                    timestamp: 0,
                },
            },
            ReleasablePackage {
                name: "package-b".to_string(),
                path: "packages/b".to_string(),
                workspace_root: ".".to_string(),
                release_type: ReleaseType::Generic,
                release: Release {
                    tag: Some(Tag {
                        sha: "sha2".to_string(),
                        name: "v2.0.0".to_string(),
                        semver: SemVer::parse("2.0.0").unwrap(),
                    }),
                    link: String::new(),
                    sha: "sha2".to_string(),
                    commits: vec![],
                    include_author: false,
                    notes: "Release notes B".to_string(),
                    timestamp: 0,
                },
            },
        ];

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        let result =
            gather_release_prs_by_branch(&packages, &mock_forge, &config)
                .await
                .unwrap();

        assert_eq!(result.len(), 1);
        assert!(result.contains_key("releasaurus-release-main"));
        assert_eq!(result["releasaurus-release-main"].len(), 2);
    }

    #[tokio::test]
    async fn gather_release_prs_creates_separate_branches_when_configured() {
        let mut config = test_helpers::create_test_config(vec![
            PackageConfig {
                name: "package-a".into(),
                path: "packages/a".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Generic),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            },
            PackageConfig {
                name: "package-b".into(),
                path: "packages/b".into(),
                workspace_root: ".".into(),
                release_type: Some(ReleaseType::Generic),
                tag_prefix: None,
                prerelease: None,
                additional_paths: None,
            },
        ]);
        config.separate_pull_requests = true;

        let packages = vec![
            ReleasablePackage {
                name: "package-a".to_string(),
                path: "packages/a".to_string(),
                workspace_root: ".".to_string(),
                release_type: ReleaseType::Generic,
                release: Release {
                    tag: Some(Tag {
                        sha: "sha1".to_string(),
                        name: "v1.0.0".to_string(),
                        semver: SemVer::parse("1.0.0").unwrap(),
                    }),
                    link: String::new(),
                    sha: "sha1".to_string(),
                    commits: vec![],
                    include_author: false,
                    notes: "Release notes A".to_string(),
                    timestamp: 0,
                },
            },
            ReleasablePackage {
                name: "package-b".to_string(),
                path: "packages/b".to_string(),
                workspace_root: ".".to_string(),
                release_type: ReleaseType::Generic,
                release: Release {
                    tag: Some(Tag {
                        sha: "sha2".to_string(),
                        name: "v2.0.0".to_string(),
                        semver: SemVer::parse("2.0.0").unwrap(),
                    }),
                    link: String::new(),
                    sha: "sha2".to_string(),
                    commits: vec![],
                    include_author: false,
                    notes: "Release notes B".to_string(),
                    timestamp: 0,
                },
            },
        ];

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        let result =
            gather_release_prs_by_branch(&packages, &mock_forge, &config)
                .await
                .unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.contains_key("releasaurus-release-main-package-a"));
        assert!(result.contains_key("releasaurus-release-main-package-b"));
        assert_eq!(result["releasaurus-release-main-package-a"].len(), 1);
        assert_eq!(result["releasaurus-release-main-package-b"].len(), 1);
    }

    #[tokio::test]
    async fn gather_release_prs_creates_changelog_file_changes() {
        let config = test_helpers::create_test_config(vec![PackageConfig {
            name: "my-package".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: None,
        }]);

        let packages = vec![ReleasablePackage {
            name: "my-package".to_string(),
            path: ".".to_string(),
            workspace_root: ".".to_string(),
            release_type: ReleaseType::Generic,
            release: Release {
                tag: Some(Tag {
                    sha: "sha1".to_string(),
                    name: "v1.0.0".to_string(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                }),
                link: String::new(),
                sha: "sha1".to_string(),
                commits: vec![],
                include_author: false,
                notes: "## v1.0.0\n\n### Features\n- New feature".to_string(),
                timestamp: 0,
            },
        }];

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        let result =
            gather_release_prs_by_branch(&packages, &mock_forge, &config)
                .await
                .unwrap();

        let prs = &result["releasaurus-release-main"];
        assert_eq!(prs.len(), 1);

        let changelog_change = prs[0]
            .file_changes
            .iter()
            .find(|fc| fc.path == "CHANGELOG.md");
        assert!(changelog_change.is_some());
        assert_eq!(
            changelog_change.unwrap().update_type,
            FileUpdateType::Prepend
        );
        assert!(changelog_change.unwrap().content.contains("## v1.0.0"));
    }

    #[tokio::test]
    async fn create_branch_release_prs_returns_error_when_merged_pr_exists() {
        let mut prs_by_branch: HashMap<String, Vec<ReleasePr>> = HashMap::new();
        prs_by_branch.insert(
            "releasaurus-release-main".to_string(),
            vec![ReleasePr {
                title: "chore: release 1.0.0".to_string(),
                body: "Release notes".to_string(),
                file_changes: vec![],
            }],
        );

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 42,
                    sha: "sha123".to_string(),
                }))
            });

        let result =
            create_branch_release_prs(prs_by_branch, &mock_forge).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_branch_release_prs_creates_new_pr_when_none_exists() {
        let mut prs_by_branch: HashMap<String, Vec<ReleasePr>> = HashMap::new();
        prs_by_branch.insert(
            "releasaurus-release-main".to_string(),
            vec![ReleasePr {
                title: "chore: release 1.0.0".to_string(),
                body: "Release notes".to_string(),
                file_changes: vec![],
            }],
        );

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_release_branch()
            .times(1)
            .returning(|_| {
                Ok(crate::forge::request::Commit {
                    sha: "new_sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_create_pr().times(1).returning(|_| {
            Ok(PullRequest {
                number: 123,
                sha: "sha123".to_string(),
            })
        });

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|_| Ok(()));

        let result =
            create_branch_release_prs(prs_by_branch, &mock_forge).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_branch_release_prs_updates_existing_pr() {
        let mut prs_by_branch: HashMap<String, Vec<ReleasePr>> = HashMap::new();
        prs_by_branch.insert(
            "releasaurus-release-main".to_string(),
            vec![ReleasePr {
                title: "chore: release 1.0.0".to_string(),
                body: "Release notes".to_string(),
                file_changes: vec![],
            }],
        );

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_get_merged_release_pr()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_release_branch()
            .times(1)
            .returning(|_| {
                Ok(crate::forge::request::Commit {
                    sha: "new_sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 99,
                    sha: "existing_sha".to_string(),
                }))
            });

        mock_forge.expect_update_pr().times(1).returning(|_| Ok(()));

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .returning(|_| Ok(()));

        let result =
            create_branch_release_prs(prs_by_branch, &mock_forge).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_releasable_packages_with_additional_paths() {
        let config = test_helpers::create_test_config(vec![PackageConfig {
            name: "my-package".into(),
            path: "packages/core".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: Some(vec!["shared/utils".to_string()]),
        }]);

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_remote_config()
            .returning(test_helpers::create_test_remote_config);

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_get_commits().times(1).returning(|_| {
            Ok(vec![
                ForgeCommit {
                    id: "abc123".to_string(),
                    link: "https://github.com/test/repo/commit/abc123"
                        .to_string(),
                    author_name: "Test Author".to_string(),
                    author_email: "test@example.com".to_string(),
                    merge_commit: false,
                    message: "feat: feature in main package".to_string(),
                    timestamp: 1000,
                    files: vec!["packages/core/src/main.rs".to_string()],
                },
                ForgeCommit {
                    id: "def456".to_string(),
                    link: "https://github.com/test/repo/commit/def456"
                        .to_string(),
                    author_name: "Test Author".to_string(),
                    author_email: "test@example.com".to_string(),
                    merge_commit: false,
                    message: "feat: feature in additional path".to_string(),
                    timestamp: 2000,
                    files: vec!["shared/utils/helper.rs".to_string()],
                },
            ])
        });

        let result = get_releasable_packages(&config, &mock_forge, None)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "my-package");
        assert!(result[0].release.tag.is_some());
        // Should include commits from both main path and additional path
        assert_eq!(result[0].release.commits.len(), 2);
    }

    #[tokio::test]
    async fn test_get_releasable_packages_additional_paths_only() {
        let config = test_helpers::create_test_config(vec![PackageConfig {
            name: "my-package".into(),
            path: "packages/core".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: Some(vec!["shared/utils".to_string()]),
        }]);

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_remote_config()
            .returning(test_helpers::create_test_remote_config);

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| Ok(None));

        // Only commits in additional_paths, not in main package path
        mock_forge.expect_get_commits().times(1).returning(|_| {
            Ok(vec![ForgeCommit {
                id: "abc123".to_string(),
                link: "https://github.com/test/repo/commit/abc123".to_string(),
                author_name: "Test Author".to_string(),
                author_email: "test@example.com".to_string(),
                merge_commit: false,
                message: "feat: feature in shared utils".to_string(),
                timestamp: 1000,
                files: vec!["shared/utils/helper.rs".to_string()],
            }])
        });

        let result = get_releasable_packages(&config, &mock_forge, None)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "my-package");
        assert!(result[0].release.tag.is_some());
        assert_eq!(result[0].release.commits.len(), 1);
    }

    #[tokio::test]
    async fn test_get_releasable_packages_ignores_commits_outside_paths() {
        let config = test_helpers::create_test_config(vec![PackageConfig {
            name: "my-package".into(),
            path: "packages/core".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: Some(vec!["shared/utils".to_string()]),
        }]);

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_remote_config()
            .returning(test_helpers::create_test_remote_config);

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_get_commits().times(1).returning(|_| {
            Ok(vec![
                ForgeCommit {
                    id: "abc123".to_string(),
                    link: "https://github.com/test/repo/commit/abc123"
                        .to_string(),
                    author_name: "Test Author".to_string(),
                    author_email: "test@example.com".to_string(),
                    merge_commit: false,
                    message: "feat: feature in other package".to_string(),
                    timestamp: 1000,
                    files: vec!["packages/other/src/main.rs".to_string()],
                },
                ForgeCommit {
                    id: "def456".to_string(),
                    link: "https://github.com/test/repo/commit/def456"
                        .to_string(),
                    author_name: "Test Author".to_string(),
                    author_email: "test@example.com".to_string(),
                    merge_commit: false,
                    message: "feat: feature in root".to_string(),
                    timestamp: 2000,
                    files: vec!["README.md".to_string()],
                },
            ])
        });

        let result = get_releasable_packages(&config, &mock_forge, None)
            .await
            .unwrap();

        // Should have no releasable packages since no commits match the paths
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_get_releasable_packages_multiple_additional_paths() {
        let config = test_helpers::create_test_config(vec![PackageConfig {
            name: "my-package".into(),
            path: "packages/core".into(),
            workspace_root: ".".into(),
            release_type: Some(ReleaseType::Generic),
            tag_prefix: None,
            prerelease: None,
            additional_paths: Some(vec![
                "shared/utils".to_string(),
                "shared/types".to_string(),
                "docs/api".to_string(),
            ]),
        }]);

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_remote_config()
            .returning(test_helpers::create_test_remote_config);

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge.expect_get_commits().times(1).returning(|_| {
            Ok(vec![
                ForgeCommit {
                    id: "abc123".to_string(),
                    link: "https://github.com/test/repo/commit/abc123"
                        .to_string(),
                    author_name: "Test Author".to_string(),
                    author_email: "test@example.com".to_string(),
                    merge_commit: false,
                    message: "feat: add utility function".to_string(),
                    timestamp: 1000,
                    files: vec!["shared/utils/helper.rs".to_string()],
                },
                ForgeCommit {
                    id: "def456".to_string(),
                    link: "https://github.com/test/repo/commit/def456"
                        .to_string(),
                    author_name: "Test Author".to_string(),
                    author_email: "test@example.com".to_string(),
                    merge_commit: false,
                    message: "feat: add new type".to_string(),
                    timestamp: 2000,
                    files: vec!["shared/types/user.rs".to_string()],
                },
                ForgeCommit {
                    id: "ghi789".to_string(),
                    link: "https://github.com/test/repo/commit/ghi789"
                        .to_string(),
                    author_name: "Test Author".to_string(),
                    author_email: "test@example.com".to_string(),
                    merge_commit: false,
                    message: "docs: update API documentation".to_string(),
                    timestamp: 3000,
                    files: vec!["docs/api/endpoints.md".to_string()],
                },
            ])
        });

        let result = get_releasable_packages(&config, &mock_forge, None)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "my-package");
        assert!(result[0].release.tag.is_some());
        // Should include commits from all additional paths
        assert_eq!(result[0].release.commits.len(), 3);
    }
}
