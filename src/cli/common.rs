//! Common functionality shared between release commands
use chrono::Utc;
use log::*;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::{collections::HashSet, path::Path};

use crate::{
    Result,
    analyzer::{Analyzer, release::Tag},
    cli::types::{ReleasablePackage, ReleasableSubPackage},
    config::package::PackageConfig,
    forge::{
        manager::ForgeManager,
        request::{CreateCommitRequest, ForgeCommit},
    },
    path_helpers::{normalize_path, package_path},
    updater::manager::UpdateManager,
};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PRMetadataFields {
    pub name: String,
    pub tag: String,
    pub notes: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PRMetadata {
    pub metadata: PRMetadataFields,
}

pub async fn start_next_release(
    packages: &[PackageConfig],
    forge_manager: &ForgeManager,
    base_branch: &str,
) -> Result<()> {
    // This is not added to changelog or tracked anywhere so we can just use
    // a fake dummy commit to trigger a patch version update
    let commits = vec![ForgeCommit {
        id: "dummy".into(),
        short_id: "dummy".into(),
        message: "fix: dummy commit".into(),
        timestamp: Utc::now().timestamp(),
        files: packages
            .iter()
            .map(|p| package_path(p, Some("dummy.txt")))
            .collect::<Vec<String>>(),
        ..ForgeCommit::default()
    }];

    let releasable_packages = get_releasable_packages_for_commits(
        packages,
        &commits,
        forge_manager,
        base_branch,
    )
    .await?;

    for pkg in releasable_packages.iter() {
        let releasable_refs: Vec<&ReleasablePackage> =
            releasable_packages.iter().collect();
        let file_changes = UpdateManager::get_package_manifest_file_changes(
            pkg,
            &releasable_refs,
        )?;

        info!("updating manifest files for package: {}", pkg.name);

        let req = CreateCommitRequest {
            target_branch: base_branch.to_string(),
            file_changes,
            message: format!(
                "chore({}): bump patch version {} - {}",
                base_branch, pkg.name, pkg.release.tag.semver
            ),
        };

        let commit = forge_manager.create_commit(req).await?;

        info!("created commit: {}", commit.sha);
    }

    Ok(())
}

pub async fn get_releasable_packages_for_commits(
    packages: &[PackageConfig],
    commits: &[ForgeCommit],
    forge_manager: &ForgeManager,
    base_branch: &str,
) -> Result<Vec<ReleasablePackage>> {
    let mut releasable_packages: Vec<ReleasablePackage> = vec![];

    for package in packages.iter() {
        let tag_prefix = package.tag_prefix()?;

        let current_tag =
            forge_manager.get_latest_tag_for_prefix(&tag_prefix).await?;

        info!(
            "processing package: \n\tname: {}, \n\tworkspace_root: {}, \n\tpath: {}, \n\ttag_prefix: {}\n\tcurrent_tag: {:?}",
            package.name,
            package.workspace_root,
            package.path,
            tag_prefix,
            current_tag
        );

        let package_commits =
            filter_commits_for_package(package, current_tag.as_ref(), commits);

        if package_commits.is_empty() {
            warn!("no processable commits found for package: {}", package.name);
            continue;
        }

        info!("processing commits for package: {}", package.name);

        let analyzer = Analyzer::new(&package.analyzer_config)?;

        if let Some(release) = analyzer.analyze(package_commits, current_tag)? {
            info!("package: {}, release: {:#?}", package.name, release);

            let release_type = package.release_type.unwrap_or_default();

            let manifest_files = UpdateManager::load_manifests_for_package(
                package,
                forge_manager,
                Some(base_branch.into()),
            )
            .await?;

            let additional_manifest_files =
                UpdateManager::load_additional_manifests_for_package(
                    package,
                    forge_manager,
                    Some(base_branch.into()),
                )
                .await?;

            let mut sub_packages = vec![];

            if let Some(sub_package_defs) = package.sub_packages.as_ref() {
                for sub in sub_package_defs.iter() {
                    let release_type = sub.release_type.unwrap_or_default();

                    let sub_path = package_path(&sub.into(), None);

                    let manifest_files =
                        UpdateManager::load_manifests_for_package(
                            &PackageConfig {
                                path: sub_path.clone(),
                                release_type: Some(release_type),
                                ..package.clone()
                            },
                            forge_manager,
                            Some(base_branch.into()),
                        )
                        .await?;

                    sub_packages.push(ReleasableSubPackage {
                        name: sub.name.clone(),
                        path: sub_path,
                        release_type,
                        manifest_files,
                    })
                }
            }

            releasable_packages.push(ReleasablePackage {
                name: package.name.clone(),
                path: package.path.clone(),
                workspace_root: package.workspace_root.clone(),
                sub_packages,
                manifest_files,
                additional_manifest_files,
                release_type,
                release: Rc::new(release),
            });
        } else {
            info!("nothing to release for package: {}", package.name);
        }
    }

    Ok(releasable_packages)
}

pub async fn get_releasable_packages(
    packages: &[PackageConfig],
    forge_manager: &ForgeManager,
    base_branch: &str,
) -> Result<Vec<ReleasablePackage>> {
    let commits =
        get_commits_for_all_packages(forge_manager, packages, base_branch)
            .await?;

    get_releasable_packages_for_commits(
        packages,
        &commits,
        forge_manager,
        base_branch,
    )
    .await
}

/// Retrieves all commits for all packages using the oldest found tag across
/// all packages. We do this once so we don't keep fetching the same commit
/// redundantly for each package.
pub async fn get_commits_for_all_packages(
    forge_manager: &ForgeManager,
    packages: &[PackageConfig],
    base_branch: &str,
) -> Result<Vec<ForgeCommit>> {
    info!("attempting to get commits for all packages at once");
    let mut starting_sha = None;
    let mut oldest_timestamp = i64::MAX;

    for package in packages.iter() {
        let tag_prefix = package.tag_prefix()?;

        if let Some(tag) =
            forge_manager.get_latest_tag_for_prefix(&tag_prefix).await?
            && let Some(timestamp) = tag.timestamp
        {
            if timestamp < oldest_timestamp {
                oldest_timestamp = timestamp;
                starting_sha = Some(tag.sha);
            }
        } else {
            // since we have a package that hasn't been tagged yet, we can't
            // determine if oldest tag for other packages will sufficiently
            // capture all the necessary commits for this package so we
            // must fall back on pull individually for each package
            warn!("found package that hasn't been tagged yet");
            starting_sha = None;
            break;
        }
    }

    if starting_sha.is_none() {
        warn!("falling back to getting commits for each package separately");
        return get_commits_for_all_packages_separately(
            forge_manager,
            packages,
            base_branch,
        )
        .await;
    }

    info!("getting commits");
    forge_manager
        .get_commits(Some(base_branch.into()), starting_sha)
        .await
}

/// Filters list of commit to just the commits pertaining to a specific package
pub fn filter_commits_for_package(
    package: &PackageConfig,
    tag: Option<&Tag>,
    commits: &[ForgeCommit],
) -> Vec<ForgeCommit> {
    let package_full_path = package_path(package, None);

    let mut package_paths = vec![package_full_path];

    if let Some(additional_paths) = package.additional_paths.clone() {
        package_paths.extend(additional_paths);
    }

    let mut package_commits: Vec<ForgeCommit> = vec![];

    for commit in commits.iter() {
        if let Some(tag) = tag
            && let Some(tag_timestamp) = tag.timestamp
            && commit.timestamp < tag_timestamp
        {
            // commit is older than package's previous release starting point
            continue;
        }
        'file_loop: for file in commit.files.iter() {
            let file_path = Path::new(file);
            for package_path in package_paths.iter() {
                // Use Cow-based normalization to avoid allocation on clean paths
                let normalized = normalize_path(package_path);
                let normalized_path = if package_path == "." {
                    Path::new("")
                } else {
                    Path::new(normalized.as_ref())
                };
                if file_path.starts_with(normalized_path) {
                    let raw_message = commit.message.to_string();
                    let split_msg = raw_message
                        .split_once("\n")
                        .map(|(m, b)| (m.to_string(), b.to_string()));

                    let (title, _body) = match split_msg {
                        Some((t, b)) => {
                            if b.is_empty() {
                                (t.trim().to_string(), None)
                            } else {
                                (
                                    t.trim().to_string(),
                                    Some(b.trim().to_string()),
                                )
                            }
                        }
                        None => (raw_message.to_string(), None),
                    };

                    debug!(
                        "{}: including commit : {} : {}",
                        package.name, commit.short_id, title
                    );

                    package_commits.push(commit.clone());
                    break 'file_loop;
                }
            }
        }
    }

    package_commits
}

/// When we can't determine a common starting point for all packages, we fall
/// back to pulling commits for each package individually and dedup by storing
/// in a HashSet
async fn get_commits_for_all_packages_separately(
    forge_manager: &ForgeManager,
    packages: &[PackageConfig],
    base_branch: &str,
) -> Result<Vec<ForgeCommit>> {
    let mut cache: HashSet<ForgeCommit> = HashSet::new();

    for package in packages.iter() {
        let tag_prefix = package.tag_prefix()?;

        let current_tag =
            forge_manager.get_latest_tag_for_prefix(&tag_prefix).await?;

        let current_sha = current_tag.as_ref().map(|t| t.sha.clone());

        info!(
            "{}: current tag sha: {:?} : fetching commits",
            package.name, current_sha
        );

        let commits = forge_manager
            .get_commits(Some(base_branch.into()), current_sha)
            .await?;

        cache.extend(commits);
    }

    let mut commits = cache.iter().cloned().collect::<Vec<ForgeCommit>>();

    commits.sort_by(|c1, c2| c1.timestamp.cmp(&c2.timestamp));

    Ok(commits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{
            package::{PackageConfigBuilder, SubPackage},
            release_type::ReleaseType,
        },
        forge::{request::Commit, traits::MockForge},
    };
    use semver::Version as SemVer;
    use std::rc::Rc;

    #[test]
    fn filter_commits_for_package_includes_commits_in_package_path() {
        let package = PackageConfigBuilder::default()
            .name("api")
            .path("packages/api")
            .tag_prefix("api-v")
            .build()
            .unwrap();
        let commits = vec![
            ForgeCommit {
                id: "abc123".into(),
                short_id: "abc123".into(),
                message: "feat: test change".into(),
                timestamp: 1000,
                files: vec!["packages/api/src/main.rs".into()],
                ..Default::default()
            },
            ForgeCommit {
                id: "def456".into(),
                short_id: "def456".into(),
                message: "feat: test change".into(),
                timestamp: 2000,
                files: vec!["packages/web/index.html".into()],
                ..Default::default()
            },
        ];

        let result = filter_commits_for_package(&package, None, &commits);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "abc123");
    }

    #[test]
    fn filter_commits_for_package_excludes_commits_before_tag_timestamp() {
        let package = PackageConfigBuilder::default()
            .name("api")
            .path("packages/api")
            .tag_prefix("api-v")
            .build()
            .unwrap();
        let tag = Tag {
            name: "api-v1.0.0".into(),
            sha: "old-sha".into(),
            semver: SemVer::parse("1.0.0").unwrap(),
            timestamp: Some(1500),
        };
        let commits = vec![
            ForgeCommit {
                id: "abc123".into(),
                short_id: "abc123".into(),
                message: "feat: test change".into(),
                timestamp: 1000,
                files: vec!["packages/api/src/main.rs".into()],
                ..Default::default()
            },
            ForgeCommit {
                id: "def456".into(),
                short_id: "def456".into(),
                message: "feat: test change".into(),
                timestamp: 2000,
                files: vec!["packages/api/src/lib.rs".into()],
                ..Default::default()
            },
        ];

        let result = filter_commits_for_package(&package, Some(&tag), &commits);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "def456");
    }

    #[test]
    fn filter_commits_for_package_handles_root_directory() {
        let package = PackageConfigBuilder::default()
            .name("root")
            .path(".")
            .tag_prefix("v")
            .build()
            .unwrap();
        let commits = vec![
            ForgeCommit {
                id: "abc123".into(),
                short_id: "abc123".into(),
                message: "feat: test change".into(),
                timestamp: 1000,
                files: vec!["src/main.rs".into()],
                ..Default::default()
            },
            ForgeCommit {
                id: "def456".into(),
                short_id: "def456".into(),
                message: "feat: test change".into(),
                timestamp: 2000,
                files: vec!["README.md".into()],
                ..Default::default()
            },
        ];

        let result = filter_commits_for_package(&package, None, &commits);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_commits_for_package_includes_additional_paths() {
        let package = PackageConfigBuilder::default()
            .name("api")
            .path("packages/api")
            .tag_prefix("api-v")
            .additional_paths(vec!["shared/utils".into()])
            .build()
            .unwrap();
        let commits = vec![
            ForgeCommit {
                id: "abc123".into(),
                short_id: "abc123".into(),
                message: "feat: test change".into(),
                timestamp: 1000,
                files: vec!["packages/api/src/main.rs".into()],
                ..Default::default()
            },
            ForgeCommit {
                id: "def456".into(),
                short_id: "def456".into(),
                message: "feat: test change".into(),
                timestamp: 2000,
                files: vec!["shared/utils/helpers.rs".into()],
                ..Default::default()
            },
            ForgeCommit {
                id: "ghi789".into(),
                short_id: "ghi789".into(),
                message: "feat: test change".into(),
                timestamp: 3000,
                files: vec!["other/file.rs".into()],
                ..Default::default()
            },
        ];

        let result = filter_commits_for_package(&package, None, &commits);

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|c| c.id == "abc123"));
        assert!(result.iter().any(|c| c.id == "def456"));
    }

    #[test]
    fn filter_commits_for_package_returns_empty_when_no_matches() {
        let package = PackageConfigBuilder::default()
            .name("api")
            .path("packages/api")
            .tag_prefix("api-v")
            .build()
            .unwrap();
        let commits = vec![ForgeCommit {
            id: "abc123".into(),
            short_id: "abc123".into(),
            message: "feat: test change".into(),
            timestamp: 1000,
            files: vec!["packages/web/index.html".into()],
            ..Default::default()
        }];

        let result = filter_commits_for_package(&package, None, &commits);

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn get_commits_for_all_packages_uses_oldest_tag() {
        let packages = vec![
            PackageConfigBuilder::default()
                .name("pkg-a")
                .path("packages/a")
                .tag_prefix("pkg-a-v")
                .release_type(ReleaseType::Node)
                .build()
                .unwrap(),
            PackageConfigBuilder::default()
                .name("pkg-b")
                .path("packages/b")
                .tag_prefix("pkg-b-v")
                .release_type(ReleaseType::Node)
                .build()
                .unwrap(),
        ];

        let mut mock = MockForge::new();
        mock.expect_repo_name().returning(|| "test-repo".into());
        mock.expect_dry_run().returning(|| false);

        mock.expect_get_latest_tag_for_prefix().returning(|prefix| {
            if prefix.contains("pkg-a") {
                Ok(Some(Tag {
                    name: "pkg-a-v1.0.0".into(),
                    sha: "sha-a".into(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                    timestamp: Some(1000),
                }))
            } else {
                Ok(Some(Tag {
                    name: "pkg-b-v1.0.0".into(),
                    sha: "sha-b".into(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                    timestamp: Some(2000),
                }))
            }
        });

        mock.expect_get_commits()
            .withf(|branch, sha| {
                branch.as_deref() == Some("main")
                    && sha.as_deref() == Some("sha-a")
            })
            .returning(|_, _| {
                Ok(vec![ForgeCommit {
                    id: "commit1".into(),
                    short_id: "commit1".into(),
                    message: "feat: test change".into(),
                    timestamp: 1500,
                    files: vec!["file.rs".into()],
                    ..Default::default()
                }])
            });

        let forge_manager = ForgeManager::new(Box::new(mock));
        let result =
            get_commits_for_all_packages(&forge_manager, &packages, "main")
                .await
                .unwrap();

        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn get_commits_for_all_packages_falls_back_when_untagged_package_exists()
     {
        let packages = vec![
            PackageConfigBuilder::default()
                .name("pkg-a")
                .path("packages/a")
                .tag_prefix("pkg-a-v")
                .release_type(ReleaseType::Node)
                .build()
                .unwrap(),
            PackageConfigBuilder::default()
                .name("pkg-b")
                .path("packages/b")
                .tag_prefix("pkg-b-v")
                .release_type(ReleaseType::Node)
                .build()
                .unwrap(),
        ];

        let mut mock = MockForge::new();
        mock.expect_repo_name().returning(|| "test-repo".into());
        mock.expect_dry_run().returning(|| false);

        mock.expect_get_latest_tag_for_prefix().returning(|prefix| {
            if prefix.contains("pkg-a") {
                Ok(Some(Tag {
                    name: "pkg-a-v1.0.0".into(),
                    sha: "sha-a".into(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                    timestamp: Some(1000),
                }))
            } else {
                Ok(None)
            }
        });

        mock.expect_get_commits().returning(|_, _| {
            Ok(vec![ForgeCommit {
                id: "commit1".into(),
                short_id: "commit1".into(),
                message: "feat: test change".into(),
                timestamp: 1500,
                files: vec!["file.rs".into()],
                ..Default::default()
            }])
        });

        let forge_manager = ForgeManager::new(Box::new(mock));
        let result =
            get_commits_for_all_packages(&forge_manager, &packages, "main")
                .await
                .unwrap();

        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn get_releasable_packages_returns_empty_when_no_commits() {
        let packages = vec![
            PackageConfigBuilder::default()
                .name("api")
                .path("packages/api")
                .tag_prefix("api-v")
                .release_type(ReleaseType::Node)
                .build()
                .unwrap(),
        ];

        let mut mock = MockForge::new();
        mock.expect_repo_name().returning(|| "test-repo".into());
        mock.expect_dry_run().returning(|| false);
        mock.expect_get_latest_tag_for_prefix()
            .returning(|_| Ok(None));
        mock.expect_get_commits().returning(|_, _| Ok(vec![]));

        let forge_manager = ForgeManager::new(Box::new(mock));
        let result = get_releasable_packages(&packages, &forge_manager, "main")
            .await
            .unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn start_next_release_creates_commits_for_packages() {
        let packages = vec![
            PackageConfigBuilder::default()
                .name("api")
                .path("packages/api")
                .tag_prefix("api-v")
                .release_type(ReleaseType::Node)
                .build()
                .unwrap(),
        ];

        let mut mock = MockForge::new();
        mock.expect_repo_name().returning(|| "test-repo".into());
        mock.expect_dry_run().returning(|| false);
        mock.expect_get_latest_tag_for_prefix().returning(|_| {
            Ok(Some(Tag {
                name: "api-v1.0.0".into(),
                sha: "sha".into(),
                semver: SemVer::parse("1.0.0").unwrap(),
                timestamp: Some(1000),
            }))
        });
        mock.expect_get_file_content().returning(|_| Ok(None));
        mock.expect_create_commit().returning(|_| {
            Ok(Commit {
                sha: "new-sha".into(),
            })
        });

        let forge_manager = ForgeManager::new(Box::new(mock));
        let result =
            start_next_release(&packages, &forge_manager, "main").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_releasable_packages_loads_sub_package_manifests() {
        // ARRANGE: Package with 2 sub-packages (different release types)
        let mut parent = PackageConfigBuilder::default()
            .name("parent")
            .path(".")
            .workspace_root(".")
            .release_type(ReleaseType::Rust)
            .tag_prefix("v")
            .build()
            .unwrap();

        parent.sub_packages = Some(vec![
            SubPackage {
                name: "api".into(),
                path: "packages/api".into(),
                release_type: Some(ReleaseType::Node),
            },
            SubPackage {
                name: "ml".into(),
                path: "packages/ml".into(),
                release_type: Some(ReleaseType::Python),
            },
        ]);

        let packages = vec![parent];

        let commits = vec![ForgeCommit {
            id: "abc123".into(),
            short_id: "abc123".into(),
            message: "feat: test change".into(),
            timestamp: 1000,
            files: vec!["src/main.rs".into()],
            ..Default::default()
        }];

        let mut mock = MockForge::new();
        mock.expect_get_latest_tag_for_prefix()
            .returning(|_| Ok(None));
        mock.expect_get_file_content().returning(|req| {
            // Return appropriate content based on file type
            if req.path.contains("Cargo.toml") {
                Ok(Some(
                    r#"[package]
name = "parent"
version = "1.0.0"
"#
                    .into(),
                ))
            } else if req.path.contains("package.json") {
                Ok(Some(r#"{"name": "api", "version": "1.0.0"}"#.into()))
            } else if req.path.contains("pyproject.toml") {
                Ok(Some(
                    r#"[project]
name = "ml"
version = "1.0.0"
"#
                    .into(),
                ))
            } else {
                Ok(None)
            }
        });

        let forge_manager = ForgeManager::new(Box::new(mock));

        // ACT: Get releasable packages
        let result = get_releasable_packages_for_commits(
            &packages,
            &commits,
            &forge_manager,
            "main",
        )
        .await
        .unwrap();

        // ASSERT: Verify sub-packages have manifest files loaded
        assert_eq!(result.len(), 1);
        let parent = &result[0];

        // Should have 2 sub-packages
        assert_eq!(parent.sub_packages.len(), 2);

        // First sub-package (Node) - should have package.json loaded
        assert_eq!(parent.sub_packages[0].name, "api");
        assert_eq!(parent.sub_packages[0].release_type, ReleaseType::Node);
        assert_eq!(parent.sub_packages[0].path, "packages/api");
        assert!(
            parent.sub_packages[0].manifest_files.is_some(),
            "Node sub-package should have manifest files loaded"
        );

        // Second sub-package (Python) - should have pyproject.toml loaded
        assert_eq!(parent.sub_packages[1].name, "ml");
        assert_eq!(parent.sub_packages[1].release_type, ReleaseType::Python);
        assert_eq!(parent.sub_packages[1].path, "packages/ml");
        assert!(
            parent.sub_packages[1].manifest_files.is_some(),
            "Python sub-package should have manifest files loaded"
        );
    }

    #[tokio::test]
    async fn get_releasable_packages_creates_sub_packages_with_shared_release()
    {
        // ARRANGE: Package with 3 sub-packages
        let mut parent = PackageConfigBuilder::default()
            .name("parent")
            .path(".")
            .workspace_root(".")
            .release_type(ReleaseType::Rust)
            .tag_prefix("v")
            .build()
            .unwrap();

        parent.sub_packages = Some(vec![
            SubPackage {
                name: "sub1".into(),
                path: "packages/sub1".into(),
                release_type: Some(ReleaseType::Node),
            },
            SubPackage {
                name: "sub2".into(),
                path: "packages/sub2".into(),
                release_type: Some(ReleaseType::Node),
            },
            SubPackage {
                name: "sub3".into(),
                path: "packages/sub3".into(),
                release_type: Some(ReleaseType::Python),
            },
        ]);

        let packages = vec![parent];

        let commits = vec![ForgeCommit {
            id: "abc123".into(),
            short_id: "abc123".into(),
            message: "feat: new feature".into(),
            timestamp: 1000,
            files: vec!["src/main.rs".into()],
            ..Default::default()
        }];

        let mut mock = MockForge::new();
        mock.expect_get_latest_tag_for_prefix()
            .returning(|_| Ok(None));
        mock.expect_get_file_content().returning(|_| Ok(None));

        let forge_manager = ForgeManager::new(Box::new(mock));

        // ACT: Get releasable packages
        let result = get_releasable_packages_for_commits(
            &packages,
            &commits,
            &forge_manager,
            "main",
        )
        .await
        .unwrap();

        // ASSERT: All sub-packages share same release
        assert_eq!(result.len(), 1);
        let parent = &result[0];
        assert_eq!(parent.sub_packages.len(), 3);

        // Verify all sub-packages reference the same release (via Rc)
        let release_ptr = Rc::as_ptr(&parent.release);
        for sub in &parent.sub_packages {
            let releasable = sub.to_releasable(parent);
            let sub_release_ptr = Rc::as_ptr(&releasable.release);
            assert_eq!(
                release_ptr, sub_release_ptr,
                "Sub-packages should share the same release"
            );
        }

        // Verify each has correct properties
        assert_eq!(parent.sub_packages[0].name, "sub1");
        assert_eq!(parent.sub_packages[1].name, "sub2");
        assert_eq!(parent.sub_packages[2].name, "sub3");
    }

    #[tokio::test]
    async fn get_releasable_packages_handles_empty_sub_packages() {
        // ARRANGE: Package with empty sub_packages array
        let mut parent = PackageConfigBuilder::default()
            .name("parent")
            .path(".")
            .workspace_root(".")
            .release_type(ReleaseType::Rust)
            .tag_prefix("v")
            .build()
            .unwrap();

        parent.sub_packages = Some(vec![]);
        let packages = vec![parent];

        let commits = vec![ForgeCommit {
            id: "abc123".into(),
            short_id: "abc123".into(),
            message: "feat: test change".into(),
            timestamp: 1000,
            files: vec!["src/main.rs".into()],
            ..Default::default()
        }];

        let mut mock = MockForge::new();
        mock.expect_get_latest_tag_for_prefix()
            .returning(|_| Ok(None));
        mock.expect_get_file_content().returning(|_| Ok(None));

        let forge_manager = ForgeManager::new(Box::new(mock));

        // ACT: Get releasable packages
        let result = get_releasable_packages_for_commits(
            &packages,
            &commits,
            &forge_manager,
            "main",
        )
        .await
        .unwrap();

        // ASSERT: Works correctly with empty sub-packages
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].sub_packages.len(), 0);
    }
}
