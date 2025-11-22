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
pub async fn execute(forge: Box<dyn Forge>) -> Result<()> {
    let mut config = forge.load_config().await?;
    let repo_name = forge.repo_name();
    let config = common::process_config(&repo_name, &mut config);

    let releasable_packages =
        get_releasable_packages(&config, forge.as_ref()).await?;

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
    let default_branch = forge.default_branch();
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
    let default_branch = forge.default_branch();

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

        let metadata = format!(
            r#"
<!--{{"metadata": {{"tag": "{}","sha": "{}"}}}}-->
"#,
            tag.name, tag.sha
        );

        // create the drop down
        let body = format!(
            "{metadata}{start_details}<summary>{}</summary>\n\n{}</details>",
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
) -> Result<Vec<ReleasablePackage>> {
    let default_branch = forge.default_branch();
    let repo_name = forge.repo_name();
    let remote_config = forge.remote_config();

    let mut manifest: Vec<ReleasablePackage> = vec![];

    let commits = common::get_commits_for_all_packages(
        forge,
        &config.packages,
        &repo_name,
    )
    .await?;

    for package in config.packages.iter() {
        let tag_prefix = common::get_tag_prefix(package, &repo_name);
        let tag = forge.get_latest_tag_for_prefix(&tag_prefix).await?;

        info!(
            "processing package: \n\tname: {}, \n\tworkspace_root: {}, \n\tpath: {}, \n\ttag_prefix: {}",
            package.name, package.workspace_root, package.path, tag_prefix
        );

        let current_tag = forge.get_latest_tag_for_prefix(&tag_prefix).await?;

        info!(
            "package_name: {}, current tag {:#?}",
            package.name, current_tag
        );

        let package_commits =
            common::filter_commits_for_package(package, tag, &commits);

        info!("processing commits for package: {}", package.name);

        let analyzer_config = common::generate_analyzer_config(
            config,
            &remote_config,
            &default_branch,
            package,
            tag_prefix.clone(),
        );

        let analyzer = Analyzer::new(analyzer_config)?;

        if let Some(release) = analyzer.analyze(package_commits, current_tag)? {
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
        forge::traits::MockForge,
        test_helpers::*,
    };
    use semver::Version as SemVer;

    fn create_releasable_package(
        name: &str,
        path: &str,
        workspace_root: &str,
        version: &str,
        release_type: ReleaseType,
    ) -> ReleasablePackage {
        ReleasablePackage {
            name: name.to_string(),
            path: path.to_string(),
            workspace_root: workspace_root.to_string(),
            release_type,
            release: Release {
                tag: Some(Tag {
                    sha: "test-sha".to_string(),
                    name: format!("v{}", version),
                    semver: SemVer::parse(version).unwrap(),
                    timestamp: 0,
                }),
                link: String::new(),
                sha: "test-sha".to_string(),
                commits: vec![],
                include_author: false,
                notes: format!("## {}\n\nRelease notes", version),
                timestamp: 0,
            },
        }
    }

    #[tokio::test]
    async fn test_gather_release_prs_single_package() {
        let packages = vec![create_releasable_package(
            "my-package",
            ".",
            ".",
            "1.0.0",
            ReleaseType::Node,
        )];

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| "main".to_string());

        mock_forge.expect_get_file_content().returning(|_| Ok(None));

        let config = create_test_config_simple(vec![(
            "my-package",
            ".",
            ReleaseType::Node,
        )]);

        let result =
            gather_release_prs_by_branch(&packages, &mock_forge, &config)
                .await
                .unwrap();

        assert_eq!(result.len(), 1);
        assert!(result.contains_key("releasaurus-release-main"));

        let prs = result.get("releasaurus-release-main").unwrap();
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].title, "chore(main): release my-package v1.0.0");
        assert!(prs[0].body.contains("v1.0.0"));
        assert!(prs[0].body.contains("<details open>"));
    }

    #[tokio::test]
    async fn test_gather_release_prs_multiple_packages_shared_branch() {
        let packages = vec![
            create_releasable_package(
                "pkg-a",
                "packages/a",
                ".",
                "1.0.0",
                ReleaseType::Node,
            ),
            create_releasable_package(
                "pkg-b",
                "packages/b",
                ".",
                "2.0.0",
                ReleaseType::Node,
            ),
        ];

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| "main".to_string());

        mock_forge.expect_get_file_content().returning(|_| Ok(None));

        let config = create_test_config_simple(vec![
            ("pkg-a", "packages/a", ReleaseType::Node),
            ("pkg-b", "packages/b", ReleaseType::Node),
        ]);

        let result =
            gather_release_prs_by_branch(&packages, &mock_forge, &config)
                .await
                .unwrap();

        assert_eq!(result.len(), 1);
        let prs = result.get("releasaurus-release-main").unwrap();
        assert_eq!(prs.len(), 2);
        assert!(prs[0].body.contains("<details>"));
        assert!(prs[1].body.contains("<details>"));
    }

    #[tokio::test]
    async fn test_gather_release_prs_separate_pull_requests() {
        let packages = vec![
            create_releasable_package(
                "pkg-a",
                "packages/a",
                ".",
                "1.0.0",
                ReleaseType::Node,
            ),
            create_releasable_package(
                "pkg-b",
                "packages/b",
                ".",
                "2.0.0",
                ReleaseType::Node,
            ),
        ];

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| "main".to_string());

        mock_forge.expect_get_file_content().returning(|_| Ok(None));

        let mut config = create_test_config_simple(vec![
            ("pkg-a", "packages/a", ReleaseType::Node),
            ("pkg-b", "packages/b", ReleaseType::Node),
        ]);
        config.separate_pull_requests = true;

        let result =
            gather_release_prs_by_branch(&packages, &mock_forge, &config)
                .await
                .unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.contains_key("releasaurus-release-main-pkg-a"));
        assert!(result.contains_key("releasaurus-release-main-pkg-b"));

        let prs_a = result.get("releasaurus-release-main-pkg-a").unwrap();
        assert_eq!(prs_a.len(), 1);
        assert!(prs_a[0].body.contains("<details open>"));

        let prs_b = result.get("releasaurus-release-main-pkg-b").unwrap();
        assert_eq!(prs_b.len(), 1);
        assert!(prs_b[0].body.contains("<details open>"));
    }

    #[tokio::test]
    async fn test_gather_release_prs_includes_changelog() {
        let packages = vec![create_releasable_package(
            "my-package",
            "packages/my-package",
            ".",
            "1.0.0",
            ReleaseType::Node,
        )];

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| "main".to_string());

        mock_forge.expect_get_file_content().returning(|_| Ok(None));

        let config = create_test_config_simple(vec![(
            "my-package",
            "packages/my-package",
            ReleaseType::Node,
        )]);

        let result =
            gather_release_prs_by_branch(&packages, &mock_forge, &config)
                .await
                .unwrap();

        let prs = result.get("releasaurus-release-main").unwrap();
        let changelog_change = prs[0]
            .file_changes
            .iter()
            .find(|fc| fc.path.contains("CHANGELOG.md"));

        assert!(changelog_change.is_some());
        let changelog_change = changelog_change.unwrap();
        assert_eq!(changelog_change.path, "packages/my-package/CHANGELOG.md");
        assert_eq!(changelog_change.update_type, FileUpdateType::Prepend);
        assert!(changelog_change.content.contains("## 1.0.0"));
    }

    #[tokio::test]
    async fn test_create_branch_release_prs_creates_new_pr() {
        let mut prs_by_branch = HashMap::new();
        prs_by_branch.insert(
            "releasaurus-release-main".to_string(),
            vec![ReleasePr {
                title: "chore(main): release my-package v1.0.0".to_string(),
                body: "Release body".to_string(),
                file_changes: vec![],
            }],
        );

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| "main".to_string());

        mock_forge
            .expect_get_merged_release_pr()
            .returning(|_| Ok(None));

        mock_forge.expect_create_release_branch().returning(|_| {
            Ok(crate::forge::request::Commit {
                sha: "branch-sha".to_string(),
            })
        });

        mock_forge
            .expect_get_open_release_pr()
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_pr()
            .returning(|_| Ok(create_test_pull_request(123, "pr-sha")));

        mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

        let result =
            create_branch_release_prs(prs_by_branch, &mock_forge).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_branch_release_prs_updates_existing_pr() {
        let mut prs_by_branch = HashMap::new();
        prs_by_branch.insert(
            "releasaurus-release-main".to_string(),
            vec![ReleasePr {
                title: "chore(main): release my-package v1.0.0".to_string(),
                body: "Release body".to_string(),
                file_changes: vec![],
            }],
        );

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| "main".to_string());

        mock_forge
            .expect_get_merged_release_pr()
            .returning(|_| Ok(None));

        mock_forge.expect_create_release_branch().returning(|_| {
            Ok(crate::forge::request::Commit {
                sha: "branch-sha".to_string(),
            })
        });

        mock_forge.expect_get_open_release_pr().returning(|_| {
            Ok(Some(create_test_pull_request(456, "existing-sha")))
        });

        mock_forge.expect_update_pr().returning(|_| Ok(()));

        mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

        let result =
            create_branch_release_prs(prs_by_branch, &mock_forge).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_branch_release_prs_fails_on_pending_release() {
        let mut prs_by_branch = HashMap::new();
        prs_by_branch.insert(
            "releasaurus-release-main".to_string(),
            vec![ReleasePr {
                title: "chore(main): release my-package v1.0.0".to_string(),
                body: "Release body".to_string(),
                file_changes: vec![],
            }],
        );

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| "main".to_string());

        mock_forge.expect_get_merged_release_pr().returning(|_| {
            Ok(Some(create_test_pull_request(789, "merged-sha")))
        });

        let result =
            create_branch_release_prs(prs_by_branch, &mock_forge).await;
        assert!(result.is_err());

        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("pending release"));
        assert!(err_msg.contains("789"));
    }

    #[tokio::test]
    async fn test_create_branch_release_prs_combines_multiple_packages() {
        let mut prs_by_branch = HashMap::new();
        prs_by_branch.insert(
            "releasaurus-release-main".to_string(),
            vec![
                ReleasePr {
                    title: "chore(main): release pkg-a v1.0.0".to_string(),
                    body: "Body A".to_string(),
                    file_changes: vec![],
                },
                ReleasePr {
                    title: "chore(main): release pkg-b v2.0.0".to_string(),
                    body: "Body B".to_string(),
                    file_changes: vec![],
                },
            ],
        );

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| "main".to_string());

        mock_forge
            .expect_get_merged_release_pr()
            .returning(|_| Ok(None));

        mock_forge.expect_create_release_branch().returning(|_| {
            Ok(crate::forge::request::Commit {
                sha: "branch-sha".to_string(),
            })
        });

        mock_forge
            .expect_get_open_release_pr()
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_pr()
            .withf(|req| {
                req.title == "chore(main): release"
                    && req.body.contains("Body A")
                    && req.body.contains("Body B")
            })
            .returning(|_| Ok(create_test_pull_request(123, "pr-sha")));

        mock_forge.expect_replace_pr_labels().returning(|_| Ok(()));

        let result =
            create_branch_release_prs(prs_by_branch, &mock_forge).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_releasable_packages_with_release() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| "main".to_string());

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_remote_config()
            .returning(create_test_remote_config);

        mock_forge.expect_get_commits().returning(|_| {
            let mut commit =
                create_test_forge_commit("abc123", "feat: new feature", 1000);
            commit.files = vec!["src/main.rs".to_string()];
            Ok(vec![commit])
        });

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(create_test_tag("v1.0.0", "1.0.0", "old-sha")))
            });

        let config = create_test_config_simple(vec![(
            "test-repo",
            ".",
            ReleaseType::Node,
        )]);

        let result =
            get_releasable_packages(&config, &mock_forge).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "test-repo");
        assert!(result[0].release.tag.is_some());
    }

    #[tokio::test]
    async fn test_get_releasable_packages_no_release() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| "main".to_string());

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_remote_config()
            .returning(create_test_remote_config);

        mock_forge.expect_get_commits().returning(|_| Ok(vec![]));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(create_test_tag("v1.0.0", "1.0.0", "current-sha")))
            });

        let config = create_test_config_simple(vec![(
            "test-repo",
            ".",
            ReleaseType::Node,
        )]);

        let result =
            get_releasable_packages(&config, &mock_forge).await.unwrap();

        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_get_releasable_packages_with_prerelease() {
        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .returning(|| "main".to_string());

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_remote_config()
            .returning(create_test_remote_config);

        mock_forge.expect_get_commits().returning(|_| {
            let mut commit =
                create_test_forge_commit("abc123", "feat: new feature", 1000);
            commit.files = vec!["src/main.rs".to_string()];
            Ok(vec![commit])
        });

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(create_test_tag("v1.0.0", "1.0.0", "old-sha")))
            });

        let mut config = create_test_config_simple(vec![(
            "test-repo",
            ".",
            ReleaseType::Node,
        )]);

        config.packages[0].prerelease = Some("alpha".to_string());

        let result =
            get_releasable_packages(&config, &mock_forge).await.unwrap();

        assert_eq!(result.len(), 1);
        let tag = result[0].release.tag.as_ref().unwrap();
        assert!(tag.semver.pre.as_str().contains("alpha"));
    }

    #[tokio::test]
    async fn test_execute_no_releasable_packages() {
        let mut mock_forge = MockForge::new();
        mock_forge.expect_load_config().returning(|| {
            Ok(create_test_config_simple(vec![(
                "test-repo",
                ".",
                ReleaseType::Node,
            )]))
        });

        mock_forge
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_default_branch()
            .returning(|| "main".to_string());

        mock_forge
            .expect_remote_config()
            .returning(create_test_remote_config);

        mock_forge.expect_get_commits().returning(|_| Ok(vec![]));

        mock_forge
            .expect_get_latest_tag_for_prefix()
            .returning(|_| {
                Ok(Some(create_test_tag("v1.0.0", "1.0.0", "current-sha")))
            });

        let result = execute(Box::new(mock_forge)).await;
        assert!(result.is_ok());
    }
}
