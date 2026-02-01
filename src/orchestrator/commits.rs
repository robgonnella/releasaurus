use std::{collections::HashSet, path::Path, rc::Rc};

use crate::{
    OrchestratorConfig, ResolvedPackage, Result,
    analyzer::release::Tag,
    forge::{manager::ForgeManager, request::ForgeCommit},
    orchestrator::package::resolved::ResolvedPackageHash,
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

    /// Retrieves all commits for all packages using the oldest found tag across
    /// all packages. We do this once so we don't keep fetching the same commit
    /// redundantly for each package.
    pub async fn get_commits_for_all_packages(
        &self,
        target: Option<&str>,
    ) -> Result<Vec<ForgeCommit>> {
        log::info!("attempting to get commits for all packages at once");

        let starting_sha = self.get_oldest_tag_sha_for_packages(target).await?;

        if starting_sha.is_none() {
            log::warn!(
                "falling back to getting commits for each package separately"
            );
            return self.get_commits_for_all_packages_separately(target).await;
        }

        log::info!("found starting sha: {:#?}", starting_sha);

        self.forge
            .get_commits(
                Some(self.orchestrator_config.base_branch.clone()),
                starting_sha,
            )
            .await
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

    /// When we can't determine a common starting point for all packages, we fall
    /// back to pulling commits for each package individually and dedup by storing
    /// in a HashSet
    async fn get_commits_for_all_packages_separately(
        &self,
        target: Option<&str>,
    ) -> Result<Vec<ForgeCommit>> {
        let mut cache: HashSet<ForgeCommit> = HashSet::new();

        for (name, package) in self.package_configs.hash().iter() {
            if let Some(target) = target
                && name != target
            {
                continue;
            }
            let current_tag = self
                .forge
                .get_latest_tag_for_prefix(&package.tag_prefix)
                .await?;

            let current_sha = current_tag.as_ref().map(|t| t.sha.clone());

            log::info!(
                "{}: current tag sha: {:?} : fetching commits",
                name,
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

    async fn get_oldest_tag_sha_for_packages(
        &self,
        target: Option<&str>,
    ) -> Result<Option<String>> {
        let mut starting_sha = None;
        let mut oldest_timestamp = i64::MAX;

        for (name, package) in self.package_configs.hash().iter() {
            if let Some(target) = target
                && name != target
            {
                continue;
            }
            if let Some(tag) = self
                .forge
                .get_latest_tag_for_prefix(&package.tag_prefix)
                .await?
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
                log::warn!("found package that hasn't been tagged yet");
                starting_sha = None;
                break;
            }
        }

        Ok(starting_sha)
    }
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::*;
    use crate::{
        OrchestratorConfig,
        analyzer::release::Tag,
        cli::{CommitModifiers, GlobalOverrides},
        config::{
            Config, package::PackageConfigBuilder, release_type::ReleaseType,
        },
        forge::{
            manager::{ForgeManager, ForgeOptions},
            request::ForgeCommitBuilder,
            traits::MockForge,
        },
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

        // Setup expectations for getting tags
        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(2)
            .returning(|prefix| {
                if prefix.contains("pkg-a") {
                    // pkg-a has newer tag (timestamp 2000)
                    Ok(Some(Tag {
                        sha: "newer-sha".to_string(),
                        timestamp: Some(2000),
                        ..Default::default()
                    }))
                } else {
                    // pkg-b has older tag (timestamp 1000)
                    Ok(Some(Tag {
                        sha: "older-sha".to_string(),
                        timestamp: Some(1000),
                        ..Default::default()
                    }))
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

        let result = commits_core.get_commits_for_all_packages(None).await;
        result.unwrap();
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

        // First package has a tag, second doesn't
        // Note: HashMap iteration order is not guaranteed, so if pkg-b (no tag)
        // comes first, the initial check breaks early (1 call), then fallback
        // makes 2 more calls (3 total). If pkg-a comes first, it's 2+2=4 calls.
        mock_forge
            .expect_get_latest_tag_for_prefix()
            .times(3..=4) // Allow either 3 or 4 calls depending on hash order
            .returning(|prefix| {
                if prefix.contains("pkg-a") {
                    Ok(Some(Tag {
                        sha: "some-sha".to_string(),
                        timestamp: Some(1000),
                        ..Default::default()
                    }))
                } else {
                    // pkg-b has no tag yet
                    Ok(None)
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

        let result = commits_core.get_commits_for_all_packages(None).await;
        result.unwrap();
    }
}
