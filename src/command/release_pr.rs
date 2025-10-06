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
        traits::{FileLoader, Forge},
    },
    result::{ReleasablePackage, Result},
    updater::framework::{Framework, updater_packages_from_manifest},
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
    file_loader: Box<dyn FileLoader>,
) -> Result<()> {
    let mut config = forge.load_config().await?;
    let repo_name = forge.repo_name();
    let config = common::process_config(&repo_name, &mut config);

    let manifest = generate_manifest(&config, forge.as_ref()).await?;

    debug!("manifest: {:#?}", manifest);

    let prs_by_branch = gather_release_prs_by_branch(
        &manifest,
        forge.as_ref(),
        file_loader.as_ref(),
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
    manifest: &[ReleasablePackage],
    forge: &dyn Forge,
    file_loader: &dyn FileLoader,
    config: &Config,
) -> Result<HashMap<String, Vec<ReleasePr>>> {
    let default_branch = forge.default_branch().await?;

    let updater_packages = updater_packages_from_manifest(manifest)?;

    let mut prs_by_branch: HashMap<String, Vec<ReleasePr>> = HashMap::new();

    for pkg in manifest.iter() {
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
        if manifest.len() == 1 || config.separate_pull_requests {
            start_details = "<details open>";
        }

        if pkg.release.tag.is_none() {
            return Err(eyre!(
                "Projected release should have a projected tag but failed to detect one. Please report this issue here: https://github.com/robgonnella/releasaurus/issues"
            ));
        }

        let tag = pkg.release.tag.clone().unwrap();

        title = format!("{title} {}", tag.name);

        // create the drop down
        let body = format!(
            "{start_details}<summary>{}</summary>\n\n{}</details>",
            tag.name, pkg.release.notes
        );

        let mut file_changes: Vec<FileChange> = vec![FileChange {
            content: format!("{}\n\n", pkg.release.notes),
            path: Path::new(&pkg.path)
                .join("CHANGELOG.md")
                .display()
                .to_string(),
            update_type: FileUpdateType::Prepend,
        }];

        let framework = Framework::from(pkg.release_type.clone());
        let updater = framework.updater();

        if let Some(more_file_changes) = updater
            .update(updater_packages.clone(), file_loader)
            .await?
        {
            file_changes.extend(more_file_changes);
        }

        let prs = prs_by_branch.get_mut(&release_branch);

        if let Some(prs) = prs {
            prs.push(ReleasePr {
                title,
                body,
                file_changes,
            })
        } else {
            prs_by_branch.insert(
                release_branch.clone(),
                vec![ReleasePr {
                    title,
                    body,
                    file_changes,
                }],
            );
        };
    }

    Ok(prs_by_branch)
}

async fn generate_manifest(
    config: &Config,
    forge: &dyn Forge,
) -> Result<Vec<ReleasablePackage>> {
    let repo_name = forge.repo_name();
    let remote_config = forge.remote_config();

    let mut manifest: Vec<ReleasablePackage> = vec![];

    for package in config.packages.iter() {
        let tag_prefix = common::get_tag_prefix(package, &repo_name);

        info!(
            "processing package: name: {}, path: {}, tag_prefix: {}",
            package.name, package.path, tag_prefix
        );

        let current_tag = forge.get_latest_tag_for_prefix(&tag_prefix).await?;

        info!("path: {}, current tag {:#?}", package.path, current_tag);

        let current_sha = current_tag.clone().map(|t| t.sha);
        let commits = forge.get_commits(&package.path, current_sha).await?;

        info!("processing commits for package: {}", package.name);

        let analyzer_config = common::generate_analyzer_config(
            config,
            &remote_config,
            tag_prefix,
        );

        let analyzer = Analyzer::new(analyzer_config)?;
        let release = analyzer.analyze(commits, current_tag)?;

        info!("package: {}, release: {:#?}", package.name, release);

        if release.is_none() {
            info!("nothing to release for package: {}", package.name);
            continue;
        }

        let release = release.unwrap();

        let release_type =
            package.release_type.clone().unwrap_or(ReleaseType::Generic);

        manifest.push(ReleasablePackage {
            name: package.name.clone(),
            path: package.path.clone(),
            release_type,
            release,
        });
    }

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::release::{Release, Tag},
        config::ReleaseType,
        forge::{
            config::RemoteConfig,
            request::{Commit, FileUpdateType, PullRequest},
            traits::{MockFileLoader, MockForge},
        },
        test_helpers,
    };
    use secrecy::SecretString;
    use semver::Version as SemVer;

    fn create_test_manifest_package(
        name: &str,
        path: &str,
        tag_name: &str,
        version: &str,
        notes: &str,
    ) -> ReleasablePackage {
        ReleasablePackage {
            name: name.to_string(),
            path: path.to_string(),
            release_type: ReleaseType::Generic,
            release: Release {
                tag: Some(Tag {
                    sha: "test-sha".to_string(),
                    name: tag_name.to_string(),
                    semver: SemVer::parse(version).unwrap(),
                }),
                link: format!(
                    "https://github.com/test/repo/releases/tag/{}",
                    tag_name
                ),
                sha: "test-sha".to_string(),
                commits: vec![],
                include_author: false,
                notes: notes.to_string(),
                timestamp: 0,
            },
        }
    }

    #[tokio::test]
    async fn test_gather_release_prs_by_branch_single_package() {
        let manifest = vec![create_test_manifest_package(
            "my-package",
            ".",
            "v1.0.0",
            "1.0.0",
            "## Changes\n- feat: new feature",
        )];

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        let mock_file_loader = MockFileLoader::new();

        let config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "my-package",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);

        let result = gather_release_prs_by_branch(
            &manifest,
            &mock_forge,
            &mock_file_loader,
            &config,
        )
        .await;

        assert!(result.is_ok());
        let prs_by_branch = result.unwrap();
        assert_eq!(prs_by_branch.len(), 1);
        assert!(prs_by_branch.contains_key("releasaurus-release-main"));

        let prs = prs_by_branch.get("releasaurus-release-main").unwrap();
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].title, "chore(main): release my-package v1.0.0");
        assert!(prs[0].body.contains("v1.0.0"));
        assert!(prs[0].body.contains("## Changes\n- feat: new feature"));
        assert!(prs[0].body.contains("<details open>"));
        assert_eq!(prs[0].file_changes.len(), 1);
        assert_eq!(prs[0].file_changes[0].path, "./CHANGELOG.md");
        assert!(matches!(
            prs[0].file_changes[0].update_type,
            FileUpdateType::Prepend
        ));
    }

    #[tokio::test]
    async fn test_gather_release_prs_by_branch_multiple_packages_single_pr() {
        let manifest = vec![
            create_test_manifest_package(
                "package-one",
                "packages/one",
                "package-one-v1.0.0",
                "1.0.0",
                "## Package One\n- feat: feature one",
            ),
            create_test_manifest_package(
                "package-two",
                "packages/two",
                "package-two-v2.0.0",
                "2.0.0",
                "## Package Two\n- feat: feature two",
            ),
        ];

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        let mock_file_loader = MockFileLoader::new();

        let config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "package-one",
                "packages/one",
                Some(ReleaseType::Generic),
                None,
            ),
            test_helpers::create_test_package_config(
                "package-two",
                "packages/two",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);

        let result = gather_release_prs_by_branch(
            &manifest,
            &mock_forge,
            &mock_file_loader,
            &config,
        )
        .await;

        assert!(result.is_ok());
        let prs_by_branch = result.unwrap();
        assert_eq!(prs_by_branch.len(), 1);

        let prs = prs_by_branch.get("releasaurus-release-main").unwrap();
        assert_eq!(prs.len(), 2);
        assert!(prs[0].body.contains("package-one-v1.0.0"));
        assert!(prs[1].body.contains("package-two-v2.0.0"));
        assert!(prs[0].body.contains("<details>"));
        assert!(prs[1].body.contains("<details>"));
    }

    #[tokio::test]
    async fn test_gather_release_prs_by_branch_separate_prs() {
        let manifest = vec![
            create_test_manifest_package(
                "package-one",
                "packages/one",
                "package-one-v1.0.0",
                "1.0.0",
                "## Package One\n- feat: feature one",
            ),
            create_test_manifest_package(
                "package-two",
                "packages/two",
                "package-two-v2.0.0",
                "2.0.0",
                "## Package Two\n- feat: feature two",
            ),
        ];

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        let mock_file_loader = MockFileLoader::new();

        let mut config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "package-one",
                "packages/one",
                Some(ReleaseType::Generic),
                None,
            ),
            test_helpers::create_test_package_config(
                "package-two",
                "packages/two",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);
        config.separate_pull_requests = true;

        let result = gather_release_prs_by_branch(
            &manifest,
            &mock_forge,
            &mock_file_loader,
            &config,
        )
        .await;

        assert!(result.is_ok());
        let prs_by_branch = result.unwrap();
        assert_eq!(prs_by_branch.len(), 2);
        assert!(
            prs_by_branch.contains_key("releasaurus-release-main-package-one")
        );
        assert!(
            prs_by_branch.contains_key("releasaurus-release-main-package-two")
        );

        let prs_one = prs_by_branch
            .get("releasaurus-release-main-package-one")
            .unwrap();
        assert_eq!(prs_one.len(), 1);
        assert!(prs_one[0].body.contains("<details open>"));

        let prs_two = prs_by_branch
            .get("releasaurus-release-main-package-two")
            .unwrap();
        assert_eq!(prs_two.len(), 1);
        assert!(prs_two[0].body.contains("<details open>"));
    }

    #[tokio::test]
    async fn test_gather_release_prs_by_branch_missing_tag_error() {
        let mut manifest_package = create_test_manifest_package(
            "my-package",
            ".",
            "v1.0.0",
            "1.0.0",
            "## Changes\n- feat: new feature",
        );
        manifest_package.release.tag = None;

        let manifest = vec![manifest_package];

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        let mock_file_loader = MockFileLoader::new();

        let config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "my-package",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);

        let result = gather_release_prs_by_branch(
            &manifest,
            &mock_forge,
            &mock_file_loader,
            &config,
        )
        .await;

        assert!(result.is_err());
        // Error occurs because tag is None - the function returns early with error
    }

    #[tokio::test]
    async fn test_gather_release_prs_by_branch_changelog_paths() {
        let manifest = vec![
            create_test_manifest_package(
                "root-package",
                ".",
                "v1.0.0",
                "1.0.0",
                "## Root changes",
            ),
            create_test_manifest_package(
                "nested-package",
                "packages/nested",
                "nested-package-v2.0.0",
                "2.0.0",
                "## Nested changes",
            ),
        ];

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        let mock_file_loader = MockFileLoader::new();

        let config = test_helpers::create_test_config(vec![
            test_helpers::create_test_package_config(
                "root-package",
                ".",
                Some(ReleaseType::Generic),
                None,
            ),
            test_helpers::create_test_package_config(
                "nested-package",
                "packages/nested",
                Some(ReleaseType::Generic),
                None,
            ),
        ]);

        let result = gather_release_prs_by_branch(
            &manifest,
            &mock_forge,
            &mock_file_loader,
            &config,
        )
        .await;

        assert!(result.is_ok());
        let prs_by_branch = result.unwrap();
        let prs = prs_by_branch.get("releasaurus-release-main").unwrap();

        assert_eq!(prs[0].file_changes[0].path, "./CHANGELOG.md");
        assert_eq!(prs[1].file_changes[0].path, "packages/nested/CHANGELOG.md");
    }

    #[tokio::test]
    async fn test_create_branch_release_prs_single_pr() {
        let mut prs_by_branch = HashMap::new();
        prs_by_branch.insert(
            "releasaurus-release-main".to_string(),
            vec![ReleasePr {
                title: "chore(main): release my-package v1.0.0".to_string(),
                body: "## Changes\n- feat: new feature".to_string(),
                file_changes: vec![FileChange {
                    path: "CHANGELOG.md".to_string(),
                    content: "## Changes\n- feat: new feature\n\n".to_string(),
                    update_type: FileUpdateType::Prepend,
                }],
            }],
        );

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_create_release_branch()
            .times(1)
            .withf(|req| {
                req.branch == "releasaurus-release-main"
                    && req.message == "chore(main): release my-package v1.0.0"
                    && req.file_changes.len() == 1
            })
            .returning(|_| {
                Ok(Commit {
                    sha: "commit-sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .withf(|req| {
                req.head_branch == "releasaurus-release-main"
                    && req.base_branch == "main"
            })
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_pr()
            .times(1)
            .withf(|req| {
                req.head_branch == "releasaurus-release-main"
                    && req.base_branch == "main"
                    && req.title == "chore(main): release my-package v1.0.0"
            })
            .returning(|_| {
                Ok(PullRequest {
                    number: 42,
                    sha: "pr-sha".to_string(),
                })
            });

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .withf(|req| {
                req.pr_number == 42 && req.labels == vec!["releasaurus:pending"]
            })
            .returning(|_| Ok(()));

        let result =
            create_branch_release_prs(prs_by_branch, &mock_forge).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_branch_release_prs_update_existing() {
        let mut prs_by_branch = HashMap::new();
        prs_by_branch.insert(
            "releasaurus-release-main".to_string(),
            vec![ReleasePr {
                title: "chore(main): release my-package v1.0.0".to_string(),
                body: "## Changes\n- feat: new feature".to_string(),
                file_changes: vec![FileChange {
                    path: "CHANGELOG.md".to_string(),
                    content: "## Changes\n- feat: new feature\n\n".to_string(),
                    update_type: FileUpdateType::Prepend,
                }],
            }],
        );

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_create_release_branch()
            .times(1)
            .returning(|_| {
                Ok(Commit {
                    sha: "commit-sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .returning(|_| {
                Ok(Some(PullRequest {
                    number: 99,
                    sha: "existing-pr-sha".to_string(),
                }))
            });

        mock_forge
            .expect_update_pr()
            .times(1)
            .withf(|req| {
                req.pr_number == 99
                    && req.title == "chore(main): release my-package v1.0.0"
            })
            .returning(|_| Ok(()));

        mock_forge
            .expect_replace_pr_labels()
            .times(1)
            .withf(|req| req.pr_number == 99)
            .returning(|_| Ok(()));

        let result =
            create_branch_release_prs(prs_by_branch, &mock_forge).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_branch_release_prs_multiple_packages_combined() {
        let mut prs_by_branch = HashMap::new();
        prs_by_branch.insert(
            "releasaurus-release-main".to_string(),
            vec![
                ReleasePr {
                    title: "chore(main): release package-one v1.0.0"
                        .to_string(),
                    body: "## Package One\n- feat: feature one".to_string(),
                    file_changes: vec![FileChange {
                        path: "packages/one/CHANGELOG.md".to_string(),
                        content: "## Package One\n- feat: feature one\n\n"
                            .to_string(),
                        update_type: FileUpdateType::Prepend,
                    }],
                },
                ReleasePr {
                    title: "chore(main): release package-two v2.0.0"
                        .to_string(),
                    body: "## Package Two\n- feat: feature two".to_string(),
                    file_changes: vec![FileChange {
                        path: "packages/two/CHANGELOG.md".to_string(),
                        content: "## Package Two\n- feat: feature two\n\n"
                            .to_string(),
                        update_type: FileUpdateType::Prepend,
                    }],
                },
            ],
        );

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_default_branch()
            .times(1)
            .returning(|| Ok("main".to_string()));

        mock_forge
            .expect_create_release_branch()
            .times(1)
            .withf(|req| {
                req.message == "chore(main): release"
                    && req.file_changes.len() == 2
            })
            .returning(|_| {
                Ok(Commit {
                    sha: "commit-sha".to_string(),
                })
            });

        mock_forge
            .expect_get_open_release_pr()
            .times(1)
            .returning(|_| Ok(None));

        mock_forge
            .expect_create_pr()
            .times(1)
            .withf(|req| {
                req.title == "chore(main): release"
                    && req.body.contains("## Package One")
                    && req.body.contains("## Package Two")
            })
            .returning(|_| {
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
            create_branch_release_prs(prs_by_branch, &mock_forge).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_generate_manifest_empty_packages() {
        let config = test_helpers::create_test_config(vec![]);

        let mut mock_forge = MockForge::new();
        mock_forge
            .expect_repo_name()
            .times(1)
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_remote_config()
            .times(1)
            .returning(|| RemoteConfig {
                host: "github.com".to_string(),
                port: None,
                scheme: "https".to_string(),
                owner: "test".to_string(),
                repo: "repo".to_string(),
                path: "test/repo".to_string(),
                token: SecretString::from("test-token".to_string()),
                commit_link_base_url: "https://github.com/test/repo/commit"
                    .to_string(),
                release_link_base_url:
                    "https://github.com/test/repo/releases/tag".to_string(),
            });

        let result = generate_manifest(&config, &mock_forge).await;
        assert!(result.is_ok());
        let manifest = result.unwrap();
        assert_eq!(manifest.len(), 0);
    }

    #[tokio::test]
    async fn test_generate_manifest_no_releases_needed() {
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
            .expect_repo_name()
            .returning(|| "test-repo".to_string());

        mock_forge
            .expect_remote_config()
            .returning(|| RemoteConfig {
                host: "github.com".to_string(),
                port: None,
                scheme: "https".to_string(),
                owner: "test".to_string(),
                repo: "repo".to_string(),
                path: "test/repo".to_string(),
                token: SecretString::from("test-token".to_string()),
                commit_link_base_url: "https://github.com/test/repo/commit"
                    .to_string(),
                release_link_base_url:
                    "https://github.com/test/repo/releases/tag".to_string(),
            });

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
            .returning(|_, _| Ok(vec![]));

        let result = generate_manifest(&config, &mock_forge).await;
        assert!(result.is_ok());
        let manifest = result.unwrap();
        assert_eq!(manifest.len(), 0);
    }
}
