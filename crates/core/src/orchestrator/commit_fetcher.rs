use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::Path,
    rc::Rc,
};

use crate::{
    forge::{
        manager::ForgeManager,
        request::{ForgeCommit, ForgeCommitPR, Tag},
    },
    packages::resolved::ResolvedPackage,
    resolver::ResolvedConfig,
    result::Result,
};

pub struct CurrentTagInfo {
    pub tag: Option<Tag>,
    pub graduating_to_stable: bool,
}

pub struct CommitFetcher {
    config: Rc<ResolvedConfig>,
    forge: Rc<ForgeManager>,
    /// Memoized PR lookups keyed by commit sha. Enrichment runs once per
    /// package, and packages share commits, so a sha is looked up once for
    /// the whole run. Every lookup targets `config.base_branch`, so the sha
    /// alone identifies the answer.
    commit_prs: RefCell<HashMap<String, Option<ForgeCommitPR>>>,
}

impl CommitFetcher {
    pub fn new(config: Rc<ResolvedConfig>, forge: Rc<ForgeManager>) -> Self {
        Self {
            config,
            forge,
            commit_prs: RefCell::new(HashMap::new()),
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
    ) -> Result<(Vec<ForgeCommit>, HashMap<String, CurrentTagInfo>)> {
        log::info!("attempting to get commits for all packages at once");

        let tags = self.collect_tags_for_packages(target).await?;
        let oldest_sha = self.oldest_tag_sha_from_map(&tags);

        let commits = if let Some(sha) = oldest_sha {
            log::info!("found starting sha: {:#?}", sha);
            self.forge
                .get_commits(Some(self.config.base_branch.clone()), Some(sha))
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
            if let Some(tag) = tag {
                if let Some(tag_timestamp) = tag.timestamp
                    && commit.timestamp < tag_timestamp
                {
                    // omit: commit is older than last release tag
                    continue;
                }

                if commit.id == tag.sha {
                    // omit: commit is previous release tag
                    continue;
                }
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

    pub async fn fetch_additional_commits_for_prerelease_aggregation(
        &self,
        pkg: &ResolvedPackage,
    ) -> Result<Vec<ForgeCommit>> {
        let mut commits = vec![];

        let latest_stable_tag = self
            .forge
            .get_latest_stable_release_tag(
                &pkg.tag_prefix,
                &self.config.base_branch,
            )
            .await?;

        if let Some(tag) = latest_stable_tag {
            // fetch previous tags starting from point of last stable release
            // so we can omit these prerelease "release" commits in the
            // changelog
            let tag_shas: HashSet<String> = self
                .forge
                .get_tags_for_prefix_since(
                    &pkg.tag_prefix,
                    &self.config.base_branch,
                    &tag.sha,
                )
                .await?
                .into_iter()
                .map(|t| t.sha)
                .collect();

            commits = self
                .forge
                .get_commits(
                    Some(self.config.base_branch.clone()),
                    Some(tag.sha.clone()),
                )
                .await?;

            commits =
                self.filter_commits_for_package(pkg, Some(&tag), &commits);

            // omit previous prereleases tagged commits from changelog
            // prevents commits like "chore: release <pkg> vX.X.X-rc.0" from
            // appearing in changelog
            commits.retain(|c| !tag_shas.contains(&c.id));
        }

        Ok(commits)
    }

    /// Attaches the PR that introduced each commit, in place.
    ///
    /// Call this *after* narrowing commits to a package, so no request is
    /// spent on a commit belonging to a different one. Note this still runs
    /// ahead of analysis, which is where `skip_shas`, skipped groups and
    /// `skip_merge_commits` drop commits — those are enriched and then
    /// discarded. No-op unless some package asked for PR links.
    pub async fn fetch_merged_commit_prs(&self, commits: &mut [ForgeCommit]) {
        if !self.config.pr_links_enabled {
            log::debug!("commit pr links are not enabled: skipping fetch");
            return;
        }
        for c in commits.iter_mut() {
            c.pr = self.commit_pr(&c.id, c.short_id.clone()).await;
        }
    }

    /// Looks up the PR that introduced `sha`, memoized per sha.
    ///
    /// A sha always has the same answer within a run, and packages share
    /// commits, so the result is cached rather than re-requested for every
    /// package a commit belongs to.
    ///
    /// PR links are cosmetic, so a failed lookup is logged and treated as
    /// "no PR" rather than failing the release.
    async fn commit_pr(
        &self,
        sha: &str,
        short_sha: String,
    ) -> Option<ForgeCommitPR> {
        // Scoped so the borrow is released before the await below.
        {
            if let Some(cached) = self.commit_prs.borrow().get(sha) {
                log::debug!("using cached PR for commit {short_sha}");
                return cached.clone();
            }
        }

        log::debug!("fetching related PR for commit {short_sha}");

        let pr = match self
            .forge
            .get_merged_pull_request_for_commit(
                sha,
                Some(self.config.base_branch.clone()),
            )
            .await
        {
            Ok(pr) => pr,
            Err(err) => {
                log::warn!(
                    "failed to fetch related PR for commit {short_sha}: \
                     {err}: omitting its PR link"
                );
                None
            }
        };

        self.commit_prs
            .borrow_mut()
            .insert(sha.to_string(), pr.clone());

        pr
    }

    /// Collects the latest tag for every (target-filtered) package in a
    /// single pass, returning a map keyed by package name.
    async fn collect_tags_for_packages(
        &self,
        target: Option<&str>,
    ) -> Result<HashMap<String, CurrentTagInfo>> {
        let mut tags = HashMap::new();
        for (name, package) in self.config.package_configs.hash().iter() {
            if let Some(target) = target
                && name != target
            {
                continue;
            }
            let tag = self
                .forge
                .get_latest_tag_for_prefix(
                    &package.tag_prefix,
                    &self.config.base_branch,
                )
                .await?;

            let graduating_to_stable = tag
                .as_ref()
                .map(|t| {
                    // We are graduating when the current tag carries a
                    // pre-release identifier but the package no longer asks
                    // for one.
                    if t.semver.pre.is_empty() {
                        // current tag does not have pre-release identifier
                        // so nothing to graduate from
                        return false;
                    }

                    // `resolve_prerelease` trims the suffix and returns `None`
                    // when it is empty, so a `Some` here always carries a real
                    // suffix. `None` is the only way to ask for stable - it
                    // covers both "no prerelease config" and "suffix cleared
                    // to graduate".
                    package.versioning_config.prerelease.is_none()
                })
                .unwrap_or_default();

            tags.insert(
                name.clone(),
                CurrentTagInfo {
                    tag,
                    graduating_to_stable,
                },
            );
        }
        Ok(tags)
    }

    /// Fetches commits per-package using pre-fetched tags, deduplicating via
    /// a HashSet. Used when a unified starting point cannot be determined
    /// (i.e. any package has no tag yet).
    async fn get_commits_for_packages_with_tags(
        &self,
        tags: &HashMap<String, CurrentTagInfo>,
    ) -> Result<Vec<ForgeCommit>> {
        let mut cache: HashSet<ForgeCommit> = HashSet::new();

        for (name, tag) in tags.iter() {
            let current_sha = tag.tag.as_ref().map(|t| t.sha.clone());

            log::info!(
                "{name}: current tag sha: {:?} : fetching commits",
                current_sha
            );

            let commits = self
                .forge
                .get_commits(Some(self.config.base_branch.clone()), current_sha)
                .await?;

            cache.extend(commits);
        }

        let mut commits = cache.iter().cloned().collect::<Vec<ForgeCommit>>();
        // restore the newest-first order guaranteed by Forge::get_commits
        commits.sort_by_key(|c| std::cmp::Reverse(c.timestamp));
        Ok(commits)
    }

    /// Returns the SHA of the oldest tag across all packages, or `None` if
    /// any package has no tag (meaning a shared starting point cannot be
    /// determined).
    fn oldest_tag_sha_from_map(
        &self,
        tags: &HashMap<String, CurrentTagInfo>,
    ) -> Option<String> {
        if tags.values().any(|t| t.tag.is_none()) {
            log::warn!("found package that hasn't been tagged yet");
            return None;
        }

        let mut oldest_timestamp = i64::MAX;
        let mut oldest_sha = None;

        for tag in tags.values().flat_map(|t| t.tag.iter()) {
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
    use std::path::PathBuf;
    use url::Url;

    use crate::{
        config::{
            Config,
            changelog::ChangelogConfig,
            overrides::{CommitModifiers, GlobalOverrides},
            package::{PackageConfig, PackageConfigBuilder},
            prerelease::{PrereleaseConfig, PrereleaseStrategy},
            release_type::ReleaseType,
            versioning::VersioningConfig,
        },
        forge::{
            manager::ForgeOptions, request::ForgeCommitBuilder,
            traits::MockForge,
        },
        resolver::ResolverBuilder,
    };

    use super::*;

    fn create_test_package(
        name: &str,
        path: &str,
    ) -> (Rc<ResolvedConfig>, ResolvedPackage) {
        let config = Rc::new(Config::default());

        let resolver = ResolverBuilder::default()
            .commit_modifiers(CommitModifiers::default())
            .compare_link_base_url(
                Url::parse("http://compare-link-base").unwrap(),
            )
            .global_overrides(GlobalOverrides::default())
            .package_overrides(HashMap::default())
            .release_link_base_url(
                Url::parse("http://release-link-base").unwrap(),
            )
            .repo_default_branch("main")
            .repo_name("test-repo")
            .toml_config(config)
            .build()
            .unwrap();

        let pkg_config = PackageConfigBuilder::default()
            .name(name)
            .path(path)
            .release_type(ReleaseType::Node)
            .build()
            .unwrap();

        let resolved_config = resolver.resolve(vec![pkg_config]).unwrap();
        let package_config =
            resolved_config.package_configs.get(name).cloned().unwrap();

        (resolved_config, package_config)
    }

    fn create_test_commit_fetcher(
        resolved_config: Rc<ResolvedConfig>,
    ) -> CommitFetcher {
        let forge = Rc::new(ForgeManager::new(
            Box::new(MockForge::new()),
            ForgeOptions { dry_run: false },
        ));

        CommitFetcher::new(resolved_config, forge)
    }

    // Helper: build a CommitFetcher wired to a single package with a custom
    // mock. Used by graduating_to_stable and aggregation tests.
    fn make_commit_fetcher_with_package(
        mock: MockForge,
        pkg_config: PackageConfig,
    ) -> CommitFetcher {
        let config = Rc::new(Config::default());

        let resolver = ResolverBuilder::default()
            .commit_modifiers(CommitModifiers::default())
            .compare_link_base_url(
                Url::parse("https://example.com/compare/").unwrap(),
            )
            .global_overrides(GlobalOverrides::default())
            .package_overrides(HashMap::default())
            .release_link_base_url(Url::parse("https://example.com/").unwrap())
            .repo_default_branch("main")
            .repo_name("test-repo")
            .toml_config(config)
            .build()
            .unwrap();

        let forge = Rc::new(ForgeManager::new(
            Box::new(mock),
            ForgeOptions { dry_run: false },
        ));

        let config = resolver.resolve(vec![pkg_config]).unwrap();

        CommitFetcher::new(config, forge)
    }

    /// Builds a fetcher whose config has `pr_links_enabled` set by routing a
    /// real `include_pr_link` through resolution, and whose forge is `mock`.
    fn create_pr_link_fetcher(enabled: bool, mock: MockForge) -> CommitFetcher {
        let resolver = ResolverBuilder::default()
            .commit_modifiers(CommitModifiers::default())
            .compare_link_base_url(
                Url::parse("http://compare-link-base").unwrap(),
            )
            .global_overrides(GlobalOverrides::default())
            .package_overrides(HashMap::default())
            .release_link_base_url(
                Url::parse("http://release-link-base").unwrap(),
            )
            .repo_default_branch("main")
            .repo_name("test-repo")
            .toml_config(Rc::new(Config::default()))
            .build()
            .unwrap();

        let pkg_config = PackageConfigBuilder::default()
            .name("test-pkg")
            .path("packages/pkg-a")
            .release_type(ReleaseType::Node)
            .changelog(ChangelogConfig {
                include_pr_link: Some(enabled),
                ..ChangelogConfig::default()
            })
            .build()
            .unwrap();

        let resolved_config = resolver.resolve(vec![pkg_config]).unwrap();
        assert_eq!(
            resolved_config.pr_links_enabled, enabled,
            "test setup did not produce the intended pr_links_enabled"
        );

        let forge = Rc::new(ForgeManager::new(
            Box::new(mock),
            ForgeOptions { dry_run: false },
        ));

        CommitFetcher::new(resolved_config, forge)
    }

    fn pr_commits(ids: &[&str]) -> Vec<ForgeCommit> {
        ids.iter()
            .map(|id| {
                ForgeCommitBuilder::default()
                    .id(*id)
                    .short_id(*id)
                    .message("feat: a feature")
                    .timestamp(1000_i64)
                    .build()
                    .unwrap()
            })
            .collect()
    }

    fn mock_returning_pr(id: &'static str) -> MockForge {
        let mut mock = MockForge::new();
        mock.expect_get_merged_pull_request_for_commit().returning(
            move |sha, _| {
                Ok(Some(ForgeCommitPR {
                    id: format!("{id}-{sha}"),
                    link: format!("https://example.com/pulls/{sha}"),
                }))
            },
        );
        mock
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

        let (config, package) = create_test_package("pkg-a", "packages/pkg-a");
        let core = create_test_commit_fetcher(config);

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

        let (config, package) = create_test_package("pkg-a", "packages/pkg-a");
        let tag = Tag {
            name: "v1.0.0".to_string(),
            timestamp: Some(2000),
            ..Default::default()
        };

        let core = create_test_commit_fetcher(config);

        let filtered =
            core.filter_commits_for_package(&package, Some(&tag), &commits);

        // Should only include commits newer than tag timestamp
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "new-commit");
    }

    #[test]
    fn omits_commit_matching_tag_sha() {
        let commits = vec![
            ForgeCommitBuilder::default()
                .id("release-sha")
                .short_id("rel")
                .message("chore(main): release v1.0.0")
                .timestamp(200)
                .files(vec!["packages/pkg-a/src/lib.rs".to_string()])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("new-commit")
                .short_id("new")
                .message("feat: new feature")
                .timestamp(300)
                .files(vec!["packages/pkg-a/src/main.rs".to_string()])
                .build()
                .unwrap(),
        ];

        let (config, package) = create_test_package("pkg-a", "packages/pkg-a");
        let tag = Tag {
            name: "v1.0.0".to_string(),
            sha: "release-sha".to_string(),
            timestamp: Some(100),
            ..Default::default()
        };

        let core = create_test_commit_fetcher(config);

        let filtered =
            core.filter_commits_for_package(&package, Some(&tag), &commits);

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

        let (config, package) = create_test_package("pkg-a", "packages/pkg-a");
        let core = create_test_commit_fetcher(config);

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

        let (config, package) = create_test_package("pkg-a", "packages/pkg-a");
        let core = create_test_commit_fetcher(config);

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

        let (config, package) = create_test_package("root-pkg", ".");
        let core = create_test_commit_fetcher(config);

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

        let (config, mut package) =
            create_test_package("pkg-a", "packages/pkg-a");
        // Add additional paths to the package
        package.normalized_additional_paths =
            vec![PathBuf::from("shared/common"), PathBuf::from("docs")];

        let core = create_test_commit_fetcher(config);

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

        let resolver = ResolverBuilder::default()
            .commit_modifiers(CommitModifiers::default())
            .compare_link_base_url(
                Url::parse("https://example.com/compare/").unwrap(),
            )
            .global_overrides(GlobalOverrides::default())
            .package_overrides(HashMap::default())
            .release_link_base_url(Url::parse("https://example.com/").unwrap())
            .repo_default_branch("main")
            .repo_name("test-repo")
            .toml_config(config)
            .build()
            .unwrap();

        let mut mock_forge = MockForge::new();

        // One tag fetch per package (collected once, not re-fetched later)
        mock_forge
            .expect_get_latest_tags_for_prefix()
            .times(2)
            .returning(|prefix, _branch, _sha| {
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

        let config =
            resolver.resolve(vec![pkg_a_config, pkg_b_config]).unwrap();

        let commit_fetcher = CommitFetcher::new(config, forge);

        let (commits, tags) = commit_fetcher
            .get_commits_for_all_packages(None)
            .await
            .unwrap();

        assert_eq!(commits.len(), 0);
        assert_eq!(tags.len(), 2);
    }

    #[tokio::test]
    async fn get_commits_falls_back_when_package_has_no_tag() {
        let config = Rc::new(Config::default());

        let resolver = ResolverBuilder::default()
            .commit_modifiers(CommitModifiers::default())
            .compare_link_base_url(
                Url::parse("https://example.com/compare/").unwrap(),
            )
            .global_overrides(GlobalOverrides::default())
            .package_overrides(HashMap::default())
            .release_link_base_url(Url::parse("https://example.com/").unwrap())
            .repo_default_branch("main")
            .repo_name("test-repo")
            .toml_config(config)
            .build()
            .unwrap();

        let mut mock_forge = MockForge::new();

        // Tags are collected once in a single pass (2 calls total).
        // The fallback fetch reuses the already-collected tags.
        mock_forge
            .expect_get_latest_tags_for_prefix()
            .times(2)
            .returning(|prefix, _branch, _sha| {
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

        let config =
            resolver.resolve(vec![pkg_a_config, pkg_b_config]).unwrap();

        let commit_fetcher = CommitFetcher::new(config, forge);

        let (commits, tags) = commit_fetcher
            .get_commits_for_all_packages(None)
            .await
            .unwrap();

        assert_eq!(commits.len(), 0);
        assert_eq!(tags.len(), 2);
    }

    // --- graduating_to_stable detection ---

    #[tokio::test]
    async fn graduating_to_stable_true_when_prerelease_tag_and_no_config() {
        let mut mock = MockForge::new();
        mock.expect_get_latest_tags_for_prefix()
            .returning(|_, _, _| {
                Ok(vec![Tag {
                    name: "v1.0.0-rc.1".to_string(),
                    semver: semver::Version::parse("1.0.0-rc.1").unwrap(),
                    sha: "sha-rc1".to_string(),
                    timestamp: Some(1000),
                }])
            });
        mock.expect_get_commits().returning(|_, _| Ok(vec![]));

        let pkg = PackageConfigBuilder::default()
            .name("test-pkg")
            .path(".")
            .release_type(ReleaseType::Node)
            .build()
            .unwrap();

        let commit_fetcher = make_commit_fetcher_with_package(mock, pkg);
        let (_, tags) = commit_fetcher
            .get_commits_for_all_packages(None)
            .await
            .unwrap();

        assert!(
            tags.get("test-pkg").unwrap().graduating_to_stable,
            "expected graduating_to_stable = true"
        );
    }

    #[tokio::test]
    async fn graduating_to_stable_false_when_stable_tag() {
        let mut mock = MockForge::new();
        mock.expect_get_latest_tags_for_prefix()
            .returning(|_, _, _| {
                Ok(vec![Tag {
                    name: "v1.0.0".to_string(),
                    semver: semver::Version::parse("1.0.0").unwrap(),
                    sha: "sha-1.0.0".to_string(),
                    timestamp: Some(1000),
                }])
            });
        mock.expect_get_commits().returning(|_, _| Ok(vec![]));

        let pkg = PackageConfigBuilder::default()
            .name("test-pkg")
            .path(".")
            .release_type(ReleaseType::Node)
            .build()
            .unwrap();

        let commit_fetcher = make_commit_fetcher_with_package(mock, pkg);
        let (_, tags) = commit_fetcher
            .get_commits_for_all_packages(None)
            .await
            .unwrap();

        assert!(
            !tags.get("test-pkg").unwrap().graduating_to_stable,
            "expected graduating_to_stable = false"
        );
    }

    #[tokio::test]
    async fn graduating_to_stable_false_when_prerelease_config_present() {
        // Current tag is a prerelease, but the package config still declares
        // a prerelease strategy — so we are NOT graduating to stable.
        let mut mock = MockForge::new();
        mock.expect_get_latest_tags_for_prefix()
            .returning(|_, _, _| {
                Ok(vec![Tag {
                    name: "v1.0.0-rc.1".to_string(),
                    semver: semver::Version::parse("1.0.0-rc.1").unwrap(),
                    sha: "sha-rc1".to_string(),
                    timestamp: Some(1000),
                }])
            });
        mock.expect_get_commits().returning(|_, _| Ok(vec![]));

        let versioning = VersioningConfig {
            prerelease: Some(PrereleaseConfig {
                suffix: "rc".to_string(),
                strategy: PrereleaseStrategy::Versioned,
            }),
            ..Default::default()
        };

        let pkg = PackageConfigBuilder::default()
            .name("test-pkg")
            .path(".")
            .release_type(ReleaseType::Node)
            .versioning(versioning)
            .build()
            .unwrap();

        let commit_fetcher = make_commit_fetcher_with_package(mock, pkg);
        let (_, tags) = commit_fetcher
            .get_commits_for_all_packages(None)
            .await
            .unwrap();

        assert!(
            !tags.get("test-pkg").unwrap().graduating_to_stable,
            "expected graduating_to_stable = false"
        );
    }

    #[tokio::test]
    async fn graduating_to_stable_false_when_no_tag() {
        let mut mock = MockForge::new();
        mock.expect_get_latest_tags_for_prefix()
            .returning(|_, _, _| Ok(vec![]));
        mock.expect_get_commits().returning(|_, _| Ok(vec![]));

        let pkg = PackageConfigBuilder::default()
            .name("test-pkg")
            .path(".")
            .release_type(ReleaseType::Node)
            .build()
            .unwrap();

        let commit_fetcher = make_commit_fetcher_with_package(mock, pkg);
        let (_, tags) = commit_fetcher
            .get_commits_for_all_packages(None)
            .await
            .unwrap();

        assert!(
            !tags.get("test-pkg").unwrap().graduating_to_stable,
            "expected graduating_to_stable = false when no tag exists"
        );
    }

    /// A cleared suffix reaches this code as `prerelease: None`, not as a
    /// `Some` carrying an empty string: `resolve_prerelease` trims and drops
    /// it during resolution. This pins that end of the contract - clearing
    /// the suffix in config is what makes the package graduate.
    #[tokio::test]
    async fn graduating_to_stable_true_when_prerelease_tag_and_empty_suffix() {
        let mut mock = MockForge::new();
        mock.expect_get_latest_tags_for_prefix()
            .returning(|_, _, _| {
                Ok(vec![Tag {
                    name: "v1.0.0-rc.1".to_string(),
                    semver: semver::Version::parse("1.0.0-rc.1").unwrap(),
                    sha: "sha-rc1".to_string(),
                    timestamp: Some(1000),
                }])
            });
        mock.expect_get_commits().returning(|_, _| Ok(vec![]));

        let versioning = VersioningConfig {
            prerelease: Some(PrereleaseConfig {
                suffix: "".to_string(),
                strategy: PrereleaseStrategy::Versioned,
            }),
            ..Default::default()
        };

        let pkg = PackageConfigBuilder::default()
            .name("test-pkg")
            .path(".")
            .release_type(ReleaseType::Node)
            .versioning(versioning)
            .build()
            .unwrap();

        let commit_fetcher = make_commit_fetcher_with_package(mock, pkg);

        // the empty suffix must not survive resolution
        assert!(
            commit_fetcher.config.package_configs.hash()["test-pkg"]
                .versioning_config
                .prerelease
                .is_none(),
            "expected an empty suffix to resolve to no prerelease config"
        );

        let (_, tags) = commit_fetcher
            .get_commits_for_all_packages(None)
            .await
            .unwrap();

        assert!(
            tags.get("test-pkg").unwrap().graduating_to_stable,
            "expected graduating_to_stable = true when suffix is empty string"
        );
    }

    // --- fetch_additional_commits_for_prerelease_aggregation ---

    #[tokio::test]
    async fn fetch_additional_returns_empty_when_no_stable_tag() {
        // Only prerelease tags exist — no stable tag to aggregate from.
        let mut mock = MockForge::new();
        mock.expect_get_latest_tags_for_prefix()
            .returning(|_, _, _| {
                Ok(vec![
                    Tag {
                        name: "v1.0.0-rc.1".to_string(),
                        semver: semver::Version::parse("1.0.0-rc.1").unwrap(),
                        sha: "sha-rc1".to_string(),
                        timestamp: None,
                    },
                    Tag {
                        name: "v1.0.0-rc.2".to_string(),
                        semver: semver::Version::parse("1.0.0-rc.2").unwrap(),
                        sha: "sha-rc2".to_string(),
                        timestamp: None,
                    },
                ])
            });

        let pkg_config = PackageConfigBuilder::default()
            .name("test-pkg")
            .path("packages/pkg-a")
            .release_type(ReleaseType::Node)
            .build()
            .unwrap();

        let commit_fetcher = make_commit_fetcher_with_package(mock, pkg_config);

        let (_, pkg) = create_test_package("test-pkg", "packages/pkg-a");

        let result = commit_fetcher
            .fetch_additional_commits_for_prerelease_aggregation(&pkg)
            .await
            .unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn fetch_additional_returns_commits_from_stable_tag_sha() {
        let stable_tag = Tag {
            name: "v1.0.0".to_string(),
            semver: semver::Version::parse("1.0.0").unwrap(),
            sha: "sha-1.0.0".to_string(),
            timestamp: Some(0),
        };

        let commit_a = ForgeCommitBuilder::default()
            .id("commit-a")
            .short_id("ca")
            .message("feat: prerelease feature")
            .timestamp(100i64)
            .files(vec!["packages/pkg-a/src/lib.rs".to_string()])
            .build()
            .unwrap();

        let commit_b = ForgeCommitBuilder::default()
            .id("commit-b")
            .short_id("cb")
            .message("fix: prerelease fix")
            .timestamp(200i64)
            .files(vec!["packages/pkg-a/src/main.rs".to_string()])
            .build()
            .unwrap();

        let commits = vec![commit_a, commit_b];

        let mut mock = MockForge::new();

        mock.expect_get_latest_tags_for_prefix()
            .returning(move |_, _, _| Ok(vec![stable_tag.clone()]));
        mock.expect_get_commits()
            .returning(move |_, _| Ok(commits.clone()));

        let pkg_config = PackageConfigBuilder::default()
            .name("test-pkg")
            .path("packages/pkg-a")
            .release_type(ReleaseType::Node)
            .build()
            .unwrap();

        let commit_fetcher = make_commit_fetcher_with_package(mock, pkg_config);
        // create resolved pkg
        let (_, pkg) = create_test_package("test-pkg", "packages/pkg-a");

        let result = commit_fetcher
            .fetch_additional_commits_for_prerelease_aggregation(&pkg)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "commit-a");
        assert_eq!(result[1].id, "commit-b");
    }

    #[tokio::test]
    async fn fetch_additional_filters_commits_by_package_path() {
        let stable_tag = Tag {
            name: "v1.0.0".to_string(),
            semver: semver::Version::parse("1.0.0").unwrap(),
            sha: "sha-1.0.0".to_string(),
            timestamp: Some(0),
        };

        let pkg_commit = ForgeCommitBuilder::default()
            .id("pkg-commit")
            .short_id("pc")
            .message("feat: change in pkg-a")
            .timestamp(100i64)
            .files(vec!["packages/pkg-a/src/lib.rs".to_string()])
            .build()
            .unwrap();

        let other_commit = ForgeCommitBuilder::default()
            .id("other-commit")
            .short_id("oc")
            .message("fix: change in other package")
            .timestamp(200i64)
            .files(vec!["packages/pkg-b/src/lib.rs".to_string()])
            .build()
            .unwrap();

        let commits = vec![pkg_commit, other_commit];

        let mut mock = MockForge::new();
        mock.expect_get_latest_tags_for_prefix()
            .returning(move |_, _, _| Ok(vec![stable_tag.clone()]));
        mock.expect_get_commits()
            .returning(move |_, _| Ok(commits.clone()));

        let pkg_config = PackageConfigBuilder::default()
            .name("test-pkg")
            .path("packages/pkg-a")
            .release_type(ReleaseType::Node)
            .build()
            .unwrap();

        let commit_fetcher = make_commit_fetcher_with_package(mock, pkg_config);
        // create resolved pkg
        let (_, pkg) = create_test_package("test-pkg", "packages/pkg-a");

        let result = commit_fetcher
            .fetch_additional_commits_for_prerelease_aggregation(&pkg)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "pkg-commit");
    }

    #[tokio::test]
    async fn fetch_merged_commit_prs_skips_lookup_when_disabled() {
        // The point of the switch: no package renders links, so no requests.
        let mut mock = MockForge::new();
        mock.expect_get_merged_pull_request_for_commit()
            .times(0)
            .returning(|_, _| Ok(None));

        let fetcher = create_pr_link_fetcher(false, mock);
        let mut commits = pr_commits(&["aaa1111", "bbb2222"]);

        fetcher.fetch_merged_commit_prs(&mut commits).await;

        assert!(commits.iter().all(|c| c.pr.is_none()));
    }

    #[tokio::test]
    async fn fetch_merged_commit_prs_attaches_pr_to_each_commit() {
        let fetcher = create_pr_link_fetcher(true, mock_returning_pr("pr"));
        let mut commits = pr_commits(&["aaa1111", "bbb2222"]);

        fetcher.fetch_merged_commit_prs(&mut commits).await;

        assert_eq!(
            commits
                .iter()
                .map(|c| c.pr.as_ref().map(|pr| pr.id.clone()))
                .collect::<Vec<_>>(),
            vec![
                Some("pr-aaa1111".to_string()),
                Some("pr-bbb2222".to_string()),
            ]
        );
    }

    #[tokio::test]
    async fn fetch_merged_commit_prs_memoizes_across_calls() {
        // Enrichment now runs once per package, and packages share commits,
        // so a sha already looked up must not be requested again.
        //
        // This also guards the memo's borrow discipline: the `RefCell` read
        // is released before the forge call and the write happens after, so a
        // second pass over the same shas would panic with a BorrowMutError if
        // either borrow were widened across the await.
        let mut mock = MockForge::new();
        mock.expect_get_merged_pull_request_for_commit()
            .times(2)
            .returning(|_, _| Ok(None));

        let fetcher = create_pr_link_fetcher(true, mock);

        for _ in 0..3 {
            let mut commits = pr_commits(&["aaa1111", "bbb2222"]);
            fetcher.fetch_merged_commit_prs(&mut commits).await;
        }
        // `times(2)` is verified on drop: 3 passes over 2 commits must still
        // issue exactly 2 lookups.
    }

    #[tokio::test]
    async fn fetch_merged_commit_prs_serves_cached_value_not_just_a_hit() {
        // A memo that returned `None` on the cache-hit path would satisfy the
        // call-count assertion above while silently dropping links, so check
        // that the second pass yields the same PR as the first.
        let fetcher = create_pr_link_fetcher(true, mock_returning_pr("pr"));

        let mut first = pr_commits(&["aaa1111"]);
        fetcher.fetch_merged_commit_prs(&mut first).await;

        let mut second = pr_commits(&["aaa1111"]);
        fetcher.fetch_merged_commit_prs(&mut second).await;

        assert_eq!(
            second[0].pr.as_ref().map(|pr| pr.id.as_str()),
            Some("pr-aaa1111")
        );
        assert_eq!(first[0].pr, second[0].pr);
    }

    #[tokio::test]
    async fn fetch_merged_commit_prs_survives_a_failed_lookup() {
        // PR links are cosmetic - a failed lookup must not fail the release.
        let mut mock = MockForge::new();
        mock.expect_get_merged_pull_request_for_commit()
            .returning(|sha, _| {
                if sha == "aaa1111" {
                    Err(crate::result::ReleasaurusError::NetworkError(
                        "boom".into(),
                    ))
                } else {
                    Ok(Some(ForgeCommitPR {
                        id: "7".into(),
                        link: "https://example.com/pulls/7".into(),
                    }))
                }
            });

        let fetcher = create_pr_link_fetcher(true, mock);
        let mut commits = pr_commits(&["aaa1111", "bbb2222"]);

        fetcher.fetch_merged_commit_prs(&mut commits).await;

        assert!(commits[0].pr.is_none(), "failed lookup should yield no PR");
        assert_eq!(
            commits[1].pr.as_ref().map(|pr| pr.id.as_str()),
            Some("7"),
            "one failure must not drop the others"
        );
    }

    #[tokio::test]
    async fn fetch_merged_commit_prs_does_not_retry_a_failed_lookup() {
        // The failure is cached too, so a later package doesn't re-hammer a
        // rate-limited forge for the same commit.
        let mut mock = MockForge::new();
        mock.expect_get_merged_pull_request_for_commit()
            .times(1)
            .returning(|_, _| {
                Err(crate::result::ReleasaurusError::RateLimitExceeded)
            });

        let fetcher = create_pr_link_fetcher(true, mock);

        for _ in 0..2 {
            let mut commits = pr_commits(&["aaa1111"]);
            fetcher.fetch_merged_commit_prs(&mut commits).await;
        }
    }
}
