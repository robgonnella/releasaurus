//! Harness for the workspace acceptance scenarios.
//!
//! Each scenario builds a throwaway git repo on disk, resolves its real
//! `releasaurus.toml`, and runs the pipeline through a [`LocalRepo`]
//! forge. That covers what a mocked `FileLoader` cannot: config
//! resolution, target discovery from real package paths, and loading
//! manifests out of a real tree.
//!
//! Assertions run against [`PRBundle::file_changes`] rather than a
//! committed tree, because `LocalRepo` without a remote logs the resolved
//! payload instead of committing it.

use std::{collections::HashMap, rc::Rc};
use tempfile::TempDir;
use url::Url;

use crate::{
    config::overrides::{CommitModifiers, GlobalOverrides},
    forge::{
        local::LocalRepo,
        manager::{ForgeManager, ForgeOptions},
        request::FileChange,
    },
    orchestrator::package_processor::PackageProcessor,
    packages::release_pr::PRBundle,
    resolver::Resolver,
    result::{ReleasaurusError, Result},
};

/// A throwaway repo, held open so its `TempDir` outlives the test.
pub struct Scenario {
    _dir: TempDir,
    processor: PackageProcessor,
}

impl Scenario {
    /// Builds a repo containing `files`, commits them as a conventional
    /// `feat:` so every configured package has something to release, and
    /// wires up the pipeline against it.
    ///
    /// `files` must include `releasaurus.toml`.
    pub async fn new(files: &[(&str, &str)]) -> Result<Self> {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();

        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        for (path, content) in files {
            let full = dir.path().join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&full, content).unwrap();
        }

        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "feat: everything", &tree, &[])
            .unwrap();

        let branch = repo.head().unwrap().shorthand().unwrap().to_string();

        let local = LocalRepo::new(dir.path(), None).await?;
        let forge = Rc::new(ForgeManager::new(
            Box::new(local),
            ForgeOptions { dry_run: false },
        ));

        let toml_config = forge.load_config(Some(branch.clone()), None).await?;
        let packages = toml_config.packages.clone();

        let resolver = Resolver::builder()
            .toml_config(Rc::new(toml_config))
            .repo_name("test-repo")
            .repo_default_branch(&branch)
            .release_link_base_url(Url::parse("https://example.com/").unwrap())
            .compare_link_base_url(
                Url::parse("https://example.com/compare/").unwrap(),
            )
            .package_overrides(HashMap::new())
            .global_overrides(GlobalOverrides::default())
            .commit_modifiers(CommitModifiers::default())
            .build()
            .unwrap();

        let resolved = resolver.resolve(packages)?;

        Ok(Self {
            _dir: dir,
            processor: PackageProcessor::new(resolved, forge),
        })
    }

    /// Runs prepare → analyze → releasable → group → bundle and returns
    /// one bundle per release branch.
    pub async fn bundles(&self) -> Result<Vec<PRBundle>> {
        let prepared = self.processor.prepare_packages(None).await?;
        let analyzed = self.processor.analyze_packages(prepared)?;
        let releasable = self.processor.releasable_packages(analyzed).await?;
        let groups = self.processor.group_releasable_packages(releasable);

        self.processor.release_pr_bundles(groups).await
    }

    /// The file changes across every bundle. Scenarios assert on this
    /// flattened view because a shared workspace file belongs to the
    /// branch, not to any one package.
    pub async fn file_changes(&self) -> Result<Vec<FileChange>> {
        let bundles = self.bundles().await?;

        Ok(bundles.into_iter().flat_map(|b| b.file_changes).collect())
    }
}

/// Unwraps the error from a rejected config.
///
/// `Scenario` holds a `PackageProcessor`, which is not `Debug`, so
/// `unwrap_err` is unavailable.
pub fn expect_rejected(result: Result<Scenario>) -> ReleasaurusError {
    match result {
        Ok(_) => panic!("expected the config to be rejected"),
        Err(e) => e,
    }
}

/// How many changes target `path`. The invariant every scenario checks is
/// that this is never above one.
pub fn count(changes: &[FileChange], path: &str) -> usize {
    changes.iter().filter(|c| c.path == path).count()
}

/// The single change for `path`, or a failure naming what was produced
/// instead.
pub fn change<'a>(changes: &'a [FileChange], path: &str) -> &'a FileChange {
    let matching: Vec<&FileChange> =
        changes.iter().filter(|c| c.path == path).collect();

    assert_eq!(
        matching.len(),
        1,
        "expected exactly one change for {path}, got {}; all paths: {:?}",
        matching.len(),
        changes.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
    );

    matching[0]
}

/// Asserts `path` is not written at all.
pub fn assert_untouched(changes: &[FileChange], path: &str) {
    assert_eq!(
        count(changes, path),
        0,
        "{path} should not have been written; all paths: {:?}",
        changes.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
    );
}

/// Convenience for the common `Cargo.lock` assertion: `name` is pinned to
/// `version`.
pub fn assert_lock_entry(lock: &str, name: &str, version: &str) {
    let entry = format!("name = \"{name}\"\nversion = \"{version}\"");

    assert!(
        lock.contains(&entry),
        "expected {name} at {version} in lock:\n{lock}"
    );
}
