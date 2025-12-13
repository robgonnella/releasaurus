//! Release pull request creation command implementation.
use color_eyre::eyre::eyre;
use log::*;
use std::{collections::HashMap, path::Path};

use crate::{
    Result,
    command::{
        common::{self, PRMetadata, PRMetadataFields},
        errors::PendingReleaseError,
        types::ReleasablePackage,
    },
    config::Config,
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, PENDING_LABEL},
        manager::ForgeManager,
        request::{
            CreateBranchRequest, CreatePrRequest, FileChange, FileUpdateType,
            GetPrRequest, PrLabelsRequest, UpdatePrRequest,
        },
    },
    updater::{generic::updater::GenericUpdater, manager::UpdateManager},
};

#[derive(Debug, Clone)]
struct ReleasePr {
    pub title: String,
    pub body: String,
    pub file_changes: Vec<FileChange>,
}

/// Analyze commits since last tags, generate changelogs, update version files,
/// and create or update release PR.
pub async fn execute(forge_manager: &ForgeManager) -> Result<()> {
    let mut config = forge_manager.load_config().await?;
    let repo_name = forge_manager.repo_name();
    let config = common::process_config(&repo_name, &mut config);

    let releasable_packages =
        common::get_releasable_packages(&config, forge_manager).await?;

    info!("releasable packages: {:#?}", releasable_packages);

    let prs_by_branch = gather_release_prs_by_branch(
        &releasable_packages,
        forge_manager,
        &config,
    )
    .await?;

    if prs_by_branch.is_empty() {
        return Ok(());
    }

    create_branch_release_prs(prs_by_branch, forge_manager).await?;

    Ok(())
}

async fn create_branch_release_prs(
    prs_by_branch: HashMap<String, Vec<ReleasePr>>,
    forge_manager: &ForgeManager,
) -> Result<()> {
    let default_branch = forge_manager.default_branch();
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

        let pending_release = forge_manager
            .get_merged_release_pr(in_process_release_req)
            .await?;

        if let Some(pr) = pending_release {
            error!("pending release: {:#?}", pr);
            return Err(PendingReleaseError {
                branch: release_branch.clone(),
                pr_number: pr.number,
            }
            .into());
        }

        info!("creating / updating release branch: {release_branch}");
        forge_manager
            .create_release_branch(CreateBranchRequest {
                branch: release_branch.clone(),
                message: title.clone(),
                file_changes,
            })
            .await?;

        info!("searching for existing pr for branch {release_branch}");
        let pr = forge_manager
            .get_open_release_pr(GetPrRequest {
                head_branch: release_branch.clone(),
                base_branch: default_branch.clone(),
            })
            .await?;

        let pr = if let Some(pr) = pr {
            forge_manager
                .update_pr(UpdatePrRequest {
                    pr_number: pr.number,
                    title,
                    body: body.join("\n"),
                })
                .await?;
            info!("updated existing release-pr: {}", pr.number);
            pr
        } else {
            let pr = forge_manager
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

        forge_manager
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
    forge_manager: &ForgeManager,
    config: &Config,
) -> Result<HashMap<String, Vec<ReleasePr>>> {
    let default_branch = forge_manager.default_branch();

    let mut prs_by_branch: HashMap<String, Vec<ReleasePr>> = HashMap::new();

    for pkg in releasable_packages.iter() {
        let mut file_changes =
            UpdateManager::get_package_manifest_file_changes(
                pkg,
                releasable_packages,
            )?;

        if let Some(additional_manifests) =
            pkg.additional_manifest_files.clone()
            && let Some(tag) = pkg.release.tag.clone()
        {
            for manifest in additional_manifests.iter() {
                if let Some(change) =
                    GenericUpdater::update_manifest(manifest, &tag.semver)
                {
                    file_changes.push(change);
                }
            }
        }

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

        let metadata = PRMetadata {
            metadata: PRMetadataFields {
                name: pkg.name.clone(),
                tag: tag.name.clone(),
                notes: pkg.release.notes.clone(),
            },
        };

        let json = serde_json::to_string(&metadata)?;

        let metadata_str = format!(
            r#"
<!--{json}-->
"#,
        );

        // create the drop down
        let body = format!(
            "{metadata_str}{start_details}<summary>{}</summary>\n\n{}</details>",
            tag.name, pkg.release.notes
        );

        let changelog_path = Path::new(&pkg.workspace_root)
            .join(&pkg.path)
            .join("CHANGELOG.md")
            .display()
            .to_string()
            .replace("\\", "/")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::release::{Release, Tag},
        config::release_type::ReleaseType,
        forge::traits::MockForge,
        test_helpers::*,
        updater::manager::ManifestFile,
    };
    use semver::Version as SemVer;

    // ===== Test Helpers =====

    /// Creates a minimal releasable package for testing
    fn releasable_package(
        name: &str,
        version: &str,
        release_type: ReleaseType,
    ) -> ReleasablePackage {
        ReleasablePackage {
            name: name.to_string(),
            path: ".".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: None,
            additional_manifest_files: None,
            release_type,
            release: Release {
                tag: Some(Tag {
                    sha: "test-sha".to_string(),
                    name: format!("v{}", version),
                    semver: SemVer::parse(version).unwrap(),
                    timestamp: None,
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

    /// Creates a forge manager with basic mocks for PR gathering tests
    fn basic_forge_manager() -> ForgeManager {
        let mut mock = MockForge::new();
        mock.expect_default_branch()
            .returning(|| "main".to_string());
        mock.expect_get_file_content().returning(|_| Ok(None));
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        ForgeManager::new(Box::new(mock))
    }

    // ===== gather_release_prs_by_branch Tests =====

    #[tokio::test]
    async fn gathers_single_pr_on_shared_branch() {
        let packages =
            vec![releasable_package("pkg", "1.0.0", ReleaseType::Node)];
        let manager = basic_forge_manager();
        let config =
            create_test_config_simple(vec![("pkg", ".", ReleaseType::Node)]);

        let result = gather_release_prs_by_branch(&packages, &manager, &config)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        let prs = &result["releasaurus-release-main"];
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].title, "chore(main): release pkg v1.0.0");
    }

    #[tokio::test]
    async fn combines_multiple_packages_on_shared_branch() {
        let packages = vec![
            releasable_package("pkg-a", "1.0.0", ReleaseType::Node),
            releasable_package("pkg-b", "2.0.0", ReleaseType::Node),
        ];
        let manager = basic_forge_manager();
        let config = create_test_config_simple(vec![
            ("pkg-a", ".", ReleaseType::Node),
            ("pkg-b", ".", ReleaseType::Node),
        ]);

        let result = gather_release_prs_by_branch(&packages, &manager, &config)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result["releasaurus-release-main"].len(), 2);
    }

    #[tokio::test]
    async fn separates_packages_when_configured() {
        let packages = vec![
            releasable_package("pkg-a", "1.0.0", ReleaseType::Node),
            releasable_package("pkg-b", "2.0.0", ReleaseType::Node),
        ];
        let manager = basic_forge_manager();
        let mut config = create_test_config_simple(vec![
            ("pkg-a", ".", ReleaseType::Node),
            ("pkg-b", ".", ReleaseType::Node),
        ]);
        config.separate_pull_requests = true;

        let result = gather_release_prs_by_branch(&packages, &manager, &config)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.contains_key("releasaurus-release-main-pkg-a"));
        assert!(result.contains_key("releasaurus-release-main-pkg-b"));
    }

    #[tokio::test]
    async fn includes_changelog_file_change() {
        let packages =
            vec![releasable_package("pkg", "1.0.0", ReleaseType::Node)];
        let manager = basic_forge_manager();
        let config =
            create_test_config_simple(vec![("pkg", ".", ReleaseType::Node)]);

        let result = gather_release_prs_by_branch(&packages, &manager, &config)
            .await
            .unwrap();

        let changelog = result["releasaurus-release-main"][0]
            .file_changes
            .iter()
            .find(|fc| fc.path.contains("CHANGELOG.md"));

        assert!(changelog.is_some());
        assert_eq!(changelog.unwrap().update_type, FileUpdateType::Prepend);
    }

    #[tokio::test]
    async fn updates_additional_manifest_files() {
        let mut pkg = releasable_package("pkg", "2.0.0", ReleaseType::Node);
        pkg.additional_manifest_files = Some(vec![ManifestFile {
            is_workspace: false,
            path: "VERSION".to_string(),
            basename: "VERSION".to_string(),
            content: r#"version = "1.0.0""#.to_string(),
        }]);

        let manager = basic_forge_manager();
        let config =
            create_test_config_simple(vec![("pkg", ".", ReleaseType::Node)]);

        let result = gather_release_prs_by_branch(&[pkg], &manager, &config)
            .await
            .unwrap();

        let version_change = result["releasaurus-release-main"][0]
            .file_changes
            .iter()
            .find(|fc| fc.path == "VERSION");

        assert!(version_change.is_some());
        assert!(version_change.unwrap().content.contains("2.0.0"));
    }

    #[tokio::test]
    async fn skips_manifests_without_version_pattern() {
        let mut pkg = releasable_package("pkg", "2.0.0", ReleaseType::Node);
        pkg.additional_manifest_files = Some(vec![ManifestFile {
            is_workspace: false,
            path: "README.md".to_string(),
            basename: "README.md".to_string(),
            content: "# My Package\n\nNo version here".to_string(),
        }]);

        let manager = basic_forge_manager();
        let config =
            create_test_config_simple(vec![("pkg", ".", ReleaseType::Node)]);

        let result = gather_release_prs_by_branch(&[pkg], &manager, &config)
            .await
            .unwrap();

        let readme = result["releasaurus-release-main"][0]
            .file_changes
            .iter()
            .find(|fc| fc.path == "README.md");

        assert!(readme.is_none());
    }

    // ===== create_branch_release_prs Tests =====

    #[tokio::test]
    async fn creates_new_pr_when_none_exists() {
        let mut prs = HashMap::new();
        prs.insert(
            "releasaurus-release-main".to_string(),
            vec![ReleasePr {
                title: "chore(main): release pkg v1.0.0".to_string(),
                body: "Body".to_string(),
                file_changes: vec![],
            }],
        );

        let mut mock = MockForge::new();
        mock.expect_default_branch()
            .returning(|| "main".to_string());
        mock.expect_get_merged_release_pr().returning(|_| Ok(None));
        mock.expect_create_release_branch().returning(|_| {
            Ok(crate::forge::request::Commit {
                sha: "sha".to_string(),
            })
        });
        mock.expect_get_open_release_pr().returning(|_| Ok(None));
        mock.expect_create_pr()
            .returning(|_| Ok(create_test_pull_request(1, "sha")));
        mock.expect_replace_pr_labels().returning(|_| Ok(()));
        mock.expect_remote_config()
            .returning(create_test_remote_config);

        let result =
            create_branch_release_prs(prs, &ForgeManager::new(Box::new(mock)))
                .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn updates_existing_pr() {
        let mut prs = HashMap::new();
        prs.insert(
            "releasaurus-release-main".to_string(),
            vec![ReleasePr {
                title: "chore(main): release pkg v1.0.0".to_string(),
                body: "Body".to_string(),
                file_changes: vec![],
            }],
        );

        let mut mock = MockForge::new();
        mock.expect_default_branch()
            .returning(|| "main".to_string());
        mock.expect_get_merged_release_pr().returning(|_| Ok(None));
        mock.expect_create_release_branch().returning(|_| {
            Ok(crate::forge::request::Commit {
                sha: "sha".to_string(),
            })
        });
        mock.expect_get_open_release_pr()
            .returning(|_| Ok(Some(create_test_pull_request(42, "sha"))));
        mock.expect_update_pr().returning(|_| Ok(()));
        mock.expect_replace_pr_labels().returning(|_| Ok(()));
        mock.expect_remote_config()
            .returning(create_test_remote_config);

        let result =
            create_branch_release_prs(prs, &ForgeManager::new(Box::new(mock)))
                .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn fails_when_pending_release_exists() {
        let mut prs = HashMap::new();
        prs.insert(
            "releasaurus-release-main".to_string(),
            vec![ReleasePr {
                title: "chore(main): release pkg v1.0.0".to_string(),
                body: "Body".to_string(),
                file_changes: vec![],
            }],
        );

        let mut mock = MockForge::new();
        mock.expect_default_branch()
            .returning(|| "main".to_string());
        mock.expect_get_merged_release_pr()
            .returning(|_| Ok(Some(create_test_pull_request(99, "sha"))));
        mock.expect_remote_config()
            .returning(create_test_remote_config);

        let result =
            create_branch_release_prs(prs, &ForgeManager::new(Box::new(mock)))
                .await;

        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("pending release"));
    }

    #[tokio::test]
    async fn combines_bodies_for_multiple_packages() {
        let mut prs = HashMap::new();
        prs.insert(
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

        let mut mock = MockForge::new();
        mock.expect_default_branch()
            .returning(|| "main".to_string());
        mock.expect_get_merged_release_pr().returning(|_| Ok(None));
        mock.expect_create_release_branch().returning(|_| {
            Ok(crate::forge::request::Commit {
                sha: "sha".to_string(),
            })
        });
        mock.expect_get_open_release_pr().returning(|_| Ok(None));
        mock.expect_create_pr()
            .withf(|req| {
                req.body.contains("Body A") && req.body.contains("Body B")
            })
            .returning(|_| Ok(create_test_pull_request(1, "sha")));
        mock.expect_replace_pr_labels().returning(|_| Ok(()));
        mock.expect_remote_config()
            .returning(create_test_remote_config);

        let result =
            create_branch_release_prs(prs, &ForgeManager::new(Box::new(mock)))
                .await;
        assert!(result.is_ok());
    }

    // ===== get_releasable_packages Tests =====

    #[tokio::test]
    async fn identifies_releasable_package() {
        let mut mock = MockForge::new();
        mock.expect_default_branch()
            .returning(|| "main".to_string());
        mock.expect_repo_name().returning(|| "repo".to_string());
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        mock.expect_get_file_content().returning(|_| Ok(None));
        mock.expect_get_commits().returning(|_| {
            let mut commit = create_test_forge_commit("abc", "feat: new", 1000);
            commit.files = vec!["main.rs".to_string()];
            Ok(vec![commit])
        });
        mock.expect_get_latest_tag_for_prefix()
            .returning(|_| Ok(Some(create_test_tag("v1.0.0", "1.0.0", "old"))));

        let config =
            create_test_config_simple(vec![("repo", ".", ReleaseType::Node)]);
        let result = common::get_releasable_packages(
            &config,
            &ForgeManager::new(Box::new(mock)),
        )
        .await
        .unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].release.tag.is_some());
    }

    #[tokio::test]
    async fn returns_empty_when_no_changes() {
        let mut mock = MockForge::new();
        mock.expect_default_branch()
            .returning(|| "main".to_string());
        mock.expect_repo_name().returning(|| "repo".to_string());
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        mock.expect_get_commits().returning(|_| Ok(vec![]));
        mock.expect_get_latest_tag_for_prefix().returning(|_| {
            Ok(Some(create_test_tag("v1.0.0", "1.0.0", "current")))
        });

        let config =
            create_test_config_simple(vec![("repo", ".", ReleaseType::Node)]);
        let result = common::get_releasable_packages(
            &config,
            &ForgeManager::new(Box::new(mock)),
        )
        .await
        .unwrap();

        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn applies_prerelease_suffix() {
        let mut mock = MockForge::new();
        mock.expect_default_branch()
            .returning(|| "main".to_string());
        mock.expect_repo_name().returning(|| "repo".to_string());
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        mock.expect_get_file_content().returning(|_| Ok(None));
        mock.expect_get_commits().returning(|_| {
            let mut commit = create_test_forge_commit("abc", "feat: new", 1000);
            commit.files = vec!["main.rs".to_string()];
            Ok(vec![commit])
        });
        mock.expect_get_latest_tag_for_prefix()
            .returning(|_| Ok(Some(create_test_tag("v1.0.0", "1.0.0", "old"))));

        let mut config =
            create_test_config_simple(vec![("repo", ".", ReleaseType::Node)]);
        config.packages[0].prerelease = Some("beta".to_string());

        let result = common::get_releasable_packages(
            &config,
            &ForgeManager::new(Box::new(mock)),
        )
        .await
        .unwrap();

        assert!(
            result[0]
                .release
                .tag
                .as_ref()
                .unwrap()
                .semver
                .pre
                .as_str()
                .contains("beta")
        );
    }

    // ===== execute Tests =====

    #[tokio::test]
    async fn succeeds_with_no_releasable_packages() {
        let mut mock = MockForge::new();
        mock.expect_load_config().returning(|| {
            Ok(create_test_config_simple(vec![(
                "repo",
                ".",
                ReleaseType::Node,
            )]))
        });
        mock.expect_repo_name().returning(|| "repo".to_string());
        mock.expect_default_branch()
            .returning(|| "main".to_string());
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        mock.expect_get_commits().returning(|_| Ok(vec![]));
        mock.expect_get_latest_tag_for_prefix().returning(|_| {
            Ok(Some(create_test_tag("v1.0.0", "1.0.0", "current")))
        });

        let result = execute(&ForgeManager::new(Box::new(mock))).await;
        assert!(result.is_ok());
    }
}
