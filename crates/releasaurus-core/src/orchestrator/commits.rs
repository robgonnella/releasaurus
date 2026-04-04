use std::{
    collections::{HashMap, HashSet},
    path::Path,
    rc::Rc,
};

use crate::{
    analyzer::release::Tag,
    error::Result,
    forge::{manager::ForgeManager, request::ForgeCommit},
    orchestrator::{
        config::OrchestratorConfig,
        package::resolved::{ResolvedPackage, ResolvedPackageHash},
    },
};

pub struct CommitsCore {
    orchestrator_config: Rc<OrchestratorConfig>,
    forge: Rc<ForgeManager>,
    package_configs: Rc<ResolvedPackageHash>,
}

impl CommitsCore {
    pub fn new(
        orchestrator_config: Rc<OrchestratorConfig>,
        forge: Rc<ForgeManager>,
        package_configs: Rc<ResolvedPackageHash>,
    ) -> Self {
        Self {
            orchestrator_config,
            forge,
            package_configs,
        }
    }

    /// Retrieves all commits for all packages along with the latest tag for
    /// each package. Uses the oldest tag across all packages as a shared
    /// starting point when possible, avoiding redundant per-package fetches.
    /// Returns `(commits, tags)` so callers can reuse the tags rather than
    /// re-querying the forge.
    pub async fn get_commits_for_all_packages(
        &self,
        target: Option<&str>,
    ) -> Result<(Vec<ForgeCommit>, HashMap<String, Option<Tag>>)> {
        log::info!("attempting to get commits for all packages at once");

        let tags = self.collect_tags_for_packages(target).await?;
        let oldest_sha = self.oldest_tag_sha_from_map(&tags);

        let commits = if let Some(sha) = oldest_sha {
            log::info!("found starting sha: {:#?}", sha);
            self.forge
                .get_commits(
                    Some(self.orchestrator_config.base_branch.clone()),
                    Some(sha),
                )
                .await?
        } else {
            log::warn!(
                "falling back to getting commits for each package separately"
            );
            self.get_commits_for_packages_with_tags(&tags).await?
        };

        Ok((commits, tags))
    }

    /// Filters list of commit to just the commits pertaining to a specific package
    pub fn filter_commits_for_package(
        &self,
        package: &ResolvedPackage,
        tag: Option<&Tag>,
        commits: &[ForgeCommit],
    ) -> Vec<ForgeCommit> {
        let mut package_paths = vec![package.normalized_full_path.clone()];
        package_paths.extend(package.normalized_additional_paths.clone());

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
                    if file_path.starts_with(package_path) {
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

                        log::debug!(
                            "{}: including commit for analysis : {} : {}",
                            package.name,
                            commit.short_id,
                            title
                        );

                        package_commits.push(commit.clone());
                        break 'file_loop;
                    }
                }
            }
        }

        package_commits
    }

    /// Collects the latest tag for every (target-filtered) package in a
    /// single pass, returning a map keyed by package name.
    async fn collect_tags_for_packages(
        &self,
        target: Option<&str>,
    ) -> Result<HashMap<String, Option<Tag>>> {
        let mut tags = HashMap::new();
        for (name, package) in self.package_configs.hash().iter() {
            if let Some(target) = target
                && name != target
            {
                continue;
            }
            let tag = self
                .forge
                .get_latest_tag_for_prefix(
                    &package.tag_prefix,
                    &self.orchestrator_config.base_branch,
                )
                .await?;
            tags.insert(name.clone(), tag);
        }
        Ok(tags)
    }

    /// Fetches commits per-package using pre-fetched tags, deduplicating via
    /// a HashSet. Used when a unified starting point cannot be determined
    /// (i.e. any package has no tag yet).
    async fn get_commits_for_packages_with_tags(
        &self,
        tags: &HashMap<String, Option<Tag>>,
    ) -> Result<Vec<ForgeCommit>> {
        let mut cache: HashSet<ForgeCommit> = HashSet::new();

        for (name, tag) in tags.iter() {
            let current_sha = tag.as_ref().map(|t| t.sha.clone());

            log::info!(
                "{name}: current tag sha: {:?} : fetching commits",
                current_sha
            );

            let commits = self
                .forge
                .get_commits(
                    Some(self.orchestrator_config.base_branch.clone()),
                    current_sha,
                )
                .await?;

            cache.extend(commits);
        }

        let mut commits = cache.iter().cloned().collect::<Vec<ForgeCommit>>();
        commits.sort_by(|c1, c2| c1.timestamp.cmp(&c2.timestamp));
        Ok(commits)
    }

    /// Returns the SHA of the oldest tag across all packages, or `None` if
    /// any package has no tag (meaning a shared starting point cannot be
    /// determined).
    fn oldest_tag_sha_from_map(
        &self,
        tags: &HashMap<String, Option<Tag>>,
    ) -> Option<String> {
        if tags.values().any(|t| t.is_none()) {
            log::warn!("found package that hasn't been tagged yet");
            return None;
        }

        let mut oldest_timestamp = i64::MAX;
        let mut oldest_sha = None;

        for tag in tags.values().flatten() {
            if let Some(ts) = tag.timestamp
                && ts < oldest_timestamp
            {
                oldest_timestamp = ts;
                oldest_sha = Some(tag.sha.clone());
            }
        }

        oldest_sha
    }
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::*;
    use crate::{
        analyzer::release::Tag,
        config::{
            Config, package::PackageConfigBuilder, release_type::ReleaseType,
        },
        forge::{
            manager::{ForgeManager, ForgeOptions},
            request::ForgeCommitBuilder,
            traits::MockForge,
        },
        orchestrator::config::{CommitModifiers, GlobalOverrides},
    };
    use std::path::PathBuf;

    fn create_test_package(name: &str, path: &str) -> ResolvedPackage {
        let config = Rc::new(Config::default());
        let pkg_config = PackageConfigBuilder::default()
            .name(name)
            .path(path)
            .release_type(ReleaseType::Node)
            .build()
            .unwrap();

        let orchestrator_config = Rc::new(
            OrchestratorConfig::builder()
                .toml_config(config)
                .repo_name("test-repo")
                .repo_default_branch("main")
                .release_link_base_url(
                    Url::parse("https://example.com/").unwrap(),
                )
                .compare_link_base_url(
                    Url::parse("https://example.com/compare/").unwrap(),
                )
                .package_overrides(std::collections::HashMap::new())
                .global_overrides(GlobalOverrides::default())
                .commit_modifiers(CommitModifiers::default())
                .build()
                .unwrap(),
        );

        ResolvedPackage::builder()
            .orchestrator_config(orchestrator_config)
            .package_config(pkg_config)
            .build()
            .unwrap()
    }

    fn create_test_commits_core() -> CommitsCore {
        let config = Rc::new(Config::default());
        let orchestrator_config = Rc::new(
            OrchestratorConfig::builder()
                .toml_config(config)
                .repo_name("test-repo")
                .repo_default_branch("main")
                .release_link_base_url(
                    Url::parse("https://example.com/").unwrap(),
                )
                .compare_link_base_url(
                    Url::parse("https://example.com/compare/").unwrap(),
                )
                .package_overrides(std::collections::HashMap::new())
                .global_overrides(GlobalOverrides::default())
                .commit_modifiers(CommitModifiers::default())
                .build()
                .unwrap(),
        );

        let forge = Rc::new(ForgeManager::new(
            Box::new(MockForge::new()),
            ForgeOptions { dry_run: false },
        ));

        let package_configs =
            Rc::new(ResolvedPackageHash::new(vec![]).unwrap());

        CommitsCore::new(orchestrator_config, forge, package_configs)
    }

    #[test]
    fn filters_commits_by_package_path() {
        let commits = vec![
            ForgeCommitBuilder::default()
                .id("commit1")
                .short_id("c1")
                .message("feat: add feature to pkg-a")
                .timestamp(1000)
                .files(vec!["packages/pkg-a/src/main.rs".to_string()])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("commit2")
                .short_id("c2")
                .message("fix: bug in pkg-b")
                .timestamp(2000)
                .files(vec!["packages/pkg-b/src/lib.rs".to_string()])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("commit3")
                .short_id("c3")
                .message("docs: update pkg-a readme")
                .timestamp(3000)
                .files(vec!["packages/pkg-a/README.md".to_string()])
                .build()
                .unwrap(),
        ];

        let package = create_test_package("pkg-a", "packages/pkg-a");
        let core = create_test_commits_core();

        let filtered =
            core.filter_commits_for_package(&package, None, &commits);

        // Should only include commits that touched packages/pkg-a
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, "commit1");
        assert_eq!(filtered[1].id, "commit3");
    }

    #[test]
    fn filters_commits_by_timestamp_when_tag_provided() {
        let commits = vec![
            ForgeCommitBuilder::default()
                .id("old-commit")
                .short_id("old")
                .message("feat: old feature")
                .timestamp(1000)
                .files(vec!["packages/pkg-a/src/old.rs".to_string()])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("new-commit")
                .short_id("new")
                .message("feat: new feature")
                .timestamp(3000)
                .files(vec!["packages/pkg-a/src/new.rs".to_string()])
                .build()
                .unwrap(),
        ];

        let package = create_test_package("pkg-a", "packages/pkg-a");
        let tag = Tag {
            name: "v1.0.0".to_string(),
            timestamp: Some(2000),
            ..Default::default()
        };

        let core = create_test_commits_core();

        let filtered =
            core.filter_commits_for_package(&package, Some(&tag), &commits);

        // Should only include commits newer than tag timestamp
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "new-commit");
    }

    #[test]
    fn includes_commit_when_any_file_matches_package_path() {
        let commits = vec![
            ForgeCommitBuilder::default()
                .id("multi-file-commit")
                .short_id("mfc")
                .message("feat: touch multiple packages")
                .timestamp(1000)
                .files(vec![
                    "packages/pkg-b/src/lib.rs".to_string(),
                    "packages/pkg-a/src/main.rs".to_string(),
                    "packages/pkg-c/README.md".to_string(),
                ])
                .build()
                .unwrap(),
        ];

        let package = create_test_package("pkg-a", "packages/pkg-a");
        let core = create_test_commits_core();

        let filtered =
            core.filter_commits_for_package(&package, None, &commits);

        // Should include the commit since one of its files matches
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "multi-file-commit");
    }

    #[test]
    fn returns_empty_when_no_commits_match_package() {
        let commits = vec![
            ForgeCommitBuilder::default()
                .id("commit1")
                .short_id("c1")
                .message("feat: work on pkg-b")
                .timestamp(1000)
                .files(vec!["packages/pkg-b/src/main.rs".to_string()])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("commit2")
                .short_id("c2")
                .message("feat: work on pkg-c")
                .timestamp(2000)
                .files(vec!["packages/pkg-c/src/lib.rs".to_string()])
                .build()
                .unwrap(),
        ];

        let package = create_test_package("pkg-a", "packages/pkg-a");
        let core = create_test_commits_core();

        let filtered =
            core.filter_commits_for_package(&package, None, &commits);

        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn handles_root_level_package() {
        let commits = vec![
            ForgeCommitBuilder::default()
                .id("root-commit")
                .short_id("rc")
                .message("feat: root level change")
                .timestamp(1000)
                .files(vec!["src/main.rs".to_string()])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("nested-commit")
                .short_id("nc")
                .message("feat: nested change")
                .timestamp(2000)
                .files(vec!["packages/nested/src/lib.rs".to_string()])
                .build()
                .unwrap(),
        ];

        let package = create_test_package("root-pkg", ".");
        let core = create_test_commits_core();

        let filtered =
            core.filter_commits_for_package(&package, None, &commits);

        // Root package should match all commits
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn includes_commits_from_additional_paths() {
        let commits = vec![
            ForgeCommitBuilder::default()
                .id("main-path-commit")
                .short_id("mpc")
                .message("feat: change in main path")
                .timestamp(1000)
                .files(vec!["packages/pkg-a/src/main.rs".to_string()])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("additional-path-commit")
                .short_id("apc")
                .message("feat: change in additional path")
                .timestamp(2000)
                .files(vec!["shared/common/utils.rs".to_string()])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("unrelated-commit")
                .short_id("uc")
                .message("feat: unrelated change")
                .timestamp(3000)
                .files(vec!["packages/pkg-b/src/lib.rs".to_string()])
                .build()
                .unwrap(),
        ];

        let mut package = create_test_package("pkg-a", "packages/pkg-a");
        // Add additional paths to the package
        package.normalized_additional_paths =
            vec![PathBuf::from("shared/common"), PathBuf::from("docs")];

        let core = create_test_commits_core();

        let filtered =
            core.filter_commits_for_package(&package, None, &commits);

        // Should include commits from both main path and additional paths
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, "main-path-commit");
        assert_eq!(filtered[1].id, "additional-path-commit");
    }

    #[tokio::test]
    async fn get_commits_uses_oldest_tag_when_all_packages_tagged() {
        let config = Rc::new(Config::default());
        let orchestrator_config = Rc::new(
            OrchestratorConfig::builder()
                .toml_config(config.clone())
                .repo_name("test-repo")
                .repo_default_branch("main")
                .release_link_base_url(
                    Url::parse("https://example.com/").unwrap(),
                )
                .compare_link_base_url(
                    Url::parse("https://example.com/compare/").unwrap(),
                )
                .package_overrides(std::collections::HashMap::new())
                .global_overrides(GlobalOverrides::default())
                .commit_modifiers(CommitModifiers::default())
                .build()
                .unwrap(),
        );

        let mut mock_forge = MockForge::new();

        // One tag fetch per package (collected once, not re-fetched later)
        mock_forge
            .expect_get_latest_tags_for_prefix()
            .times(2)
            .returning(|prefix, _branch| {
                if prefix.contains("pkg-a") {
                    // pkg-a has newer tag (timestamp 2000)
                    Ok(vec![Tag {
                        sha: "newer-sha".to_string(),
                        timestamp: Some(2000),
                        ..Default::default()
                    }])
                } else {
                    // pkg-b has older tag (timestamp 1000)
                    Ok(vec![Tag {
                        sha: "older-sha".to_string(),
                        timestamp: Some(1000),
                        ..Default::default()
                    }])
                }
            });

        // Should use the older SHA
        mock_forge
            .expect_get_commits()
            .times(1)
            .withf(|branch, sha| {
                branch.as_ref().unwrap() == "main"
                    && sha.as_ref().unwrap() == "older-sha"
            })
            .returning(|_, _| Ok(vec![]));

        let forge = Rc::new(ForgeManager::new(
            Box::new(mock_forge),
            ForgeOptions { dry_run: false },
        ));

        // Create two packages
        let pkg_a_config = PackageConfigBuilder::default()
            .name("pkg-a")
            .path("packages/pkg-a")
            .release_type(ReleaseType::Node)
            .build()
            .unwrap();

        let pkg_b_config = PackageConfigBuilder::default()
            .name("pkg-b")
            .path("packages/pkg-b")
            .release_type(ReleaseType::Node)
            .build()
            .unwrap();

        let pkg_a = ResolvedPackage::builder()
            .orchestrator_config(Rc::clone(&orchestrator_config))
            .package_config(pkg_a_config)
            .build()
            .unwrap();

        let pkg_b = ResolvedPackage::builder()
            .orchestrator_config(Rc::clone(&orchestrator_config))
            .package_config(pkg_b_config)
            .build()
            .unwrap();

        let package_configs =
            Rc::new(ResolvedPackageHash::new(vec![pkg_a, pkg_b]).unwrap());

        let commits_core = CommitsCore::new(
            Rc::clone(&orchestrator_config),
            forge,
            package_configs,
        );

        let (commits, tags) = commits_core
            .get_commits_for_all_packages(None)
            .await
            .unwrap();
        assert_eq!(commits.len(), 0);
        assert_eq!(tags.len(), 2);
    }

    #[tokio::test]
    async fn get_commits_falls_back_when_package_has_no_tag() {
        let config = Rc::new(Config::default());
        let orchestrator_config = Rc::new(
            OrchestratorConfig::builder()
                .toml_config(config.clone())
                .repo_name("test-repo")
                .repo_default_branch("main")
                .release_link_base_url(
                    Url::parse("https://example.com/").unwrap(),
                )
                .compare_link_base_url(
                    Url::parse("https://example.com/compare/").unwrap(),
                )
                .package_overrides(std::collections::HashMap::new())
                .global_overrides(GlobalOverrides::default())
                .commit_modifiers(CommitModifiers::default())
                .build()
                .unwrap(),
        );

        let mut mock_forge = MockForge::new();

        // Tags are collected once in a single pass (2 calls total).
        // The fallback fetch reuses the already-collected tags.
        mock_forge
            .expect_get_latest_tags_for_prefix()
            .times(2)
            .returning(|prefix, _branch| {
                if prefix.contains("pkg-a") {
                    Ok(vec![Tag {
                        sha: "some-sha".to_string(),
                        timestamp: Some(1000),
                        ..Default::default()
                    }])
                } else {
                    // pkg-b has no tag yet
                    Ok(vec![])
                }
            });

        // Should fall back to getting commits per package (2 calls)
        mock_forge
            .expect_get_commits()
            .times(2)
            .returning(|_, _| Ok(vec![]));

        let forge = Rc::new(ForgeManager::new(
            Box::new(mock_forge),
            ForgeOptions { dry_run: false },
        ));

        let pkg_a_config = PackageConfigBuilder::default()
            .name("pkg-a")
            .path("packages/pkg-a")
            .release_type(ReleaseType::Node)
            .build()
            .unwrap();

        let pkg_b_config = PackageConfigBuilder::default()
            .name("pkg-b")
            .path("packages/pkg-b")
            .release_type(ReleaseType::Node)
            .build()
            .unwrap();

        let pkg_a = ResolvedPackage::builder()
            .orchestrator_config(Rc::clone(&orchestrator_config))
            .package_config(pkg_a_config)
            .build()
            .unwrap();

        let pkg_b = ResolvedPackage::builder()
            .orchestrator_config(Rc::clone(&orchestrator_config))
            .package_config(pkg_b_config)
            .build()
            .unwrap();

        let package_configs =
            Rc::new(ResolvedPackageHash::new(vec![pkg_a, pkg_b]).unwrap());

        let commits_core = CommitsCore::new(
            Rc::clone(&orchestrator_config),
            forge,
            package_configs,
        );

        let (commits, tags) = commits_core
            .get_commits_for_all_packages(None)
            .await
            .unwrap();
        assert_eq!(commits.len(), 0);
        assert_eq!(tags.len(), 2);
    }
}
