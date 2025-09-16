//! Git repository operations for release automation workflows.
//!
//! Handles cloning, authentication, branch management, commit operations,
//! tagging, and version history analysis with token-based authentication.
use color_eyre::eyre::{Context, eyre};
use git2::{Commit, Oid, RemoteCallbacks, Sort, TreeWalkMode};
use glob::Pattern;
use log::*;
use regex::Regex;
use reqwest::Url;
use secrecy::ExposeSecret;
use std::path::{Path, PathBuf};

use crate::{
    analyzer::types::Tag, forge::config::RemoteConfig, result::Result,
};

/// Default upstream remote name after renaming "origin".
const DEFAULT_UPSTREAM_REMOTE: &str = "upstream";

/// Cache file name for changed files.
const CHANGED_FILES_CACHE: &str = "changed_files_cache";

/// Git repository wrapper with release automation functionality.
pub struct Repository {
    /// The default branch name of the remote repository (e.g., "main" or "master").
    pub default_branch: String,
    /// Remote repository configuration including authentication details.
    config: RemoteConfig,
    /// The underlying git2 repository instance.
    repo: git2::Repository,
    changed_files_cache_path: PathBuf,
}

/// Create Git authentication callbacks for username/token auth.
fn get_auth_callbacks<'r>(user: String, token: String) -> RemoteCallbacks<'r> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(move |_url, _username, _allowed| {
        git2::Cred::userpass_plaintext(&user, &token)
    });
    callbacks
}

impl Repository {
    /// Clone remote repository to local path with shallow clone and authentication.
    pub fn new(
        local_path: &Path,
        clone_depth: u64,
        config: RemoteConfig,
    ) -> Result<Self> {
        let repo_url =
            format!("{}://{}/{}", config.scheme, config.host, config.path);

        let url = Url::parse(repo_url.as_str())?;
        let git_config = git2::Config::open_default()?.snapshot()?;
        let user = git_config.get_str("user.name")?;
        let token = config.token.expose_secret().to_string();

        // setup callbacks for authentication
        let callbacks = get_auth_callbacks(user.into(), token.clone());

        // Sets a maximum depth commits when cloning to prevent cloning
        // thousands of commits. This is one of the reasons this project works
        // best on repos that enforce linear commit histories. If we tried
        // a depth of 250 on something like torvalds/linux repo we would get
        // many thousands of commits due to the non-linear history of that repo
        let mut fetch_options = git2::FetchOptions::new();
        if clone_depth > 0 {
            fetch_options.depth(clone_depth as i32);
        }
        fetch_options.remote_callbacks(callbacks);

        let mut builder = git2::build::RepoBuilder::new();
        let repo = builder
            .fetch_options(fetch_options)
            .clone(url.as_str(), local_path)?;

        repo.remote_rename("origin", DEFAULT_UPSTREAM_REMOTE)?;

        // setup callbacks for authentication
        let callbacks = get_auth_callbacks(user.into(), token.clone());
        let mut remote = repo.find_remote(DEFAULT_UPSTREAM_REMOTE)?;
        remote.connect_auth(git2::Direction::Fetch, Some(callbacks), None)?;

        let buf = remote.default_branch()?;
        let default_branch =
            buf.as_str().ok_or(eyre!("failed to get default branch"))?;
        let default_branch = default_branch.replace("refs/heads/", "");

        drop(remote);

        let changed_files_cache_path = repo
            .path()
            .join(env!("CARGO_PKG_NAME"))
            .join(CHANGED_FILES_CACHE);

        Ok(Self {
            repo,
            config,
            default_branch,
            changed_files_cache_path,
        })
    }

    /// Create repository instance from existing local git repository (for testing).
    #[cfg(test)]
    pub fn from_local(
        local_path: &Path,
        config: RemoteConfig,
        default_branch: String,
    ) -> Result<Self> {
        let repo = git2::Repository::open(local_path)?;

        let changed_files_cache_path = repo
            .path()
            .join(env!("CARGO_PKG_NAME"))
            .join(CHANGED_FILES_CACHE);

        Ok(Self {
            repo,
            config,
            default_branch,
            changed_files_cache_path,
        })
    }

    /// Normalize glob pattern to match git diff paths.
    /// take from git-cliff-core with slight modification
    fn normalize_pattern(pattern: Pattern) -> Result<Pattern> {
        let star_added = match pattern.as_str().chars().last() {
            Some('/' | '\\') => Pattern::new(&format!("{pattern}**"))
                .wrap_err("failed to add glob (**) to pattern")?,
            _ => pattern,
        };
        match star_added.as_str().strip_prefix("./") {
            Some(stripped) => Pattern::new(stripped)
                .wrap_err("failed to strip ./ prefix from pattern"),
            None => Ok(star_added),
        }
    }

    /// Calculate changed files for commit without using cache.
    /// take from git-cliff-core with slight modification
    fn get_changed_files_no_cache(
        &self,
        commit: &Commit,
    ) -> Result<Vec<PathBuf>> {
        let mut changed_files = Vec::new();
        if let Ok(prev_commit) = commit.parent(0) {
            // Compare the current commit with the previous commit to get the
            // changed files.
            // libgit2 does not provide a way to get the changed files directly,
            // so the full diff is calculated here.
            if let Ok(diff) = self.repo.diff_tree_to_tree(
                commit.tree().ok().as_ref(),
                prev_commit.tree().ok().as_ref(),
                None,
            ) {
                changed_files.extend(diff.deltas().filter_map(|delta| {
                    delta.new_file().path().map(PathBuf::from)
                }));
            }
        } else {
            // If there is no parent, it is the first commit.
            // So get all the files in the tree.
            if let Ok(tree) = commit.tree() {
                tree.walk(TreeWalkMode::PreOrder, |dir, entry| {
                    let kind = entry.kind().unwrap_or(git2::ObjectType::Any);

                    if kind != git2::ObjectType::Blob {
                        return 0;
                    }

                    let name = entry.name().unwrap_or("");

                    if name.is_empty() {
                        return 1;
                    }

                    let entry_path = if dir == "," {
                        name.to_string()
                    } else {
                        format!("{dir}/{name}")
                    };

                    changed_files.push(entry_path.into());

                    0
                })
                .wrap_err(
                    "failed to get the changed files of the first commit",
                )?;
            }
        }

        Ok(changed_files)
    }

    /// Get changed files for commit with caching for performance.
    /// take from git-cliff-core with slight modification
    fn get_changed_files(&self, commit: &Commit) -> Result<Vec<PathBuf>> {
        // Cache key is generated from the repository path and commit id
        let cache_key = format!("commit_id:{}", commit.id());

        // Check the cache first.
        {
            if let Ok(result) =
                cacache::read_sync(&self.changed_files_cache_path, &cache_key)
                && let Ok((files, _)) = bincode::decode_from_slice(
                    &result,
                    bincode::config::standard(),
                )
            {
                return Ok(files);
            }
        }

        // If the cache is not found, calculate the result and set in cache
        let result = self.get_changed_files_no_cache(commit);
        match bincode::encode_to_vec(
            self.get_changed_files_no_cache(commit)?,
            bincode::config::standard(),
        ) {
            Ok(v) => {
                if let Err(e) = cacache::write_sync_with_algo(
                    cacache::Algorithm::Xxh3,
                    &self.changed_files_cache_path,
                    cache_key,
                    v,
                ) {
                    error!(
                        "Failed to set cache for repo {:?}: {e}",
                        self.repo.path()
                    );
                }
            }
            Err(e) => {
                error!(
                    "Failed to serialize cache for repo {:?}: {e}",
                    self.repo.path(),
                );
            }
        }

        result
    }

    /// Check if commit should be retained based on changed files and patterns.
    /// take from git-cliff-core with slight modification
    fn should_retain_commit(
        &self,
        commit: &Commit,
        include_patterns: &Option<Vec<Pattern>>,
    ) -> Result<bool> {
        let changed_files = self.get_changed_files(commit)?;
        match include_patterns {
            Some(include_pattern) => {
                // check if the commit has any changed files that match any of
                // the include patterns and none of the exclude patterns.
                Ok(changed_files.iter().any(|path| {
                    include_pattern
                        .iter()
                        .any(|pattern| pattern.matches_path(path))
                }))
            }
            None => Ok(true),
        }
    }

    /// Parse and return commits sorted by time.
    /// take from git-cliff-core with slight modification
    pub fn commits(
        &self,
        range: Option<&str>,
        include_path: Option<Vec<Pattern>>,
    ) -> Result<Vec<Commit<'_>>> {
        let mut revwalk = self.repo.revwalk()?;

        revwalk.set_sorting(Sort::TIME)?;

        if let Some(range) = range {
            revwalk.push_range(range)?;
        } else {
            revwalk.push_head()?;
        }

        let mut commits: Vec<Commit> = revwalk
            .filter_map(|id| id.ok())
            .filter_map(|id| self.repo.find_commit(id).ok())
            .collect();

        if include_path.is_some() {
            let include_patterns = include_path.map(|patterns| {
                patterns
                    .into_iter()
                    .map(|p| Self::normalize_pattern(p.clone()).unwrap_or(p))
                    .collect()
            });
            commits.retain(|commit| {
                self.should_retain_commit(commit, &include_patterns)
                    .unwrap_or(false)
            });
        }
        Ok(commits)
    }

    /// Find latest git tag matching the given prefix.
    pub fn get_latest_tag(&self, prefix: &str) -> Result<Option<Tag>> {
        let regex_prefix = format!(r"^{}", prefix);
        let tag_prefix_regex = Regex::new(&regex_prefix)?;
        let references = self
            .repo
            .references()?
            .filter_map(|r| r.ok())
            .collect::<Vec<git2::Reference>>();

        let mut commits: Vec<(git2::Commit, Tag)> = vec![];
        for reference in references.iter() {
            if reference.is_tag()
                && let Some(name) = reference.name()
                && let Some(stripped) = name.strip_prefix("refs/tags/")
                && tag_prefix_regex.is_match(stripped)
            {
                let commit = reference.peel_to_commit()?;
                let semver = semver::Version::parse(
                    tag_prefix_regex.replace_all(stripped, "").as_ref(),
                )?;

                let tag = Tag {
                    sha: commit.id().to_string(),
                    name: stripped.to_string(),
                    semver,
                };
                commits.push((commit, tag));
            }
        }

        if commits.is_empty() {
            return Ok(None);
        }

        // sort commits by time descending so the first one should contain
        // the latest tag
        commits.sort_by(|(c1, _), (c2, _)| c2.time().cmp(&c1.time()));

        let (_, tag) = commits[0].clone();

        Ok(Some(tag))
    }

    /// Create new branch from current HEAD (force creates, overwrites existing).
    pub fn create_branch(&self, branch: &str) -> Result<()> {
        info!("creating branch: {branch}");
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        self.repo.branch(branch, &commit, true)?;
        Ok(())
    }

    /// Switch to branch and update working directory.
    pub fn switch_branch(&self, branch: &str) -> Result<()> {
        info!("switching to branch: {branch}");
        let ref_name = format!("refs/heads/{}", branch);
        let target_obj = self.repo.revparse_single(&ref_name)?;
        self.repo.checkout_tree(&target_obj, None)?;
        self.repo.set_head(&ref_name)?;
        Ok(())
    }

    /// Add all changed files to git index (equivalent to `git add .`).
    pub fn add_all(&self) -> Result<()> {
        debug!("adding changed files to index");
        let mut index = self.repo.index()?;
        index.add_all(["."], git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    /// Create commit with staged changes and specified message.
    pub fn commit(&self, msg: &str) -> Result<()> {
        debug!("committing changes with msg: {msg}");
        let config = self.repo.config()?.snapshot()?;
        let user = config.get_str("user.name")?;
        let email = config.get_str("user.email")?;
        debug!("using committer: user: {user}, email: {email}");
        let mut index = self.repo.index()?;
        let oid = index.write_tree()?;
        let tree = self.repo.find_tree(oid)?;
        let parent_commit = self.repo.head()?.peel_to_commit()?;
        let committer = git2::Signature::now(user, email)?;
        self.repo.commit(
            Some("HEAD"),
            &committer,
            &committer,
            msg,
            &tree,
            &[&parent_commit],
        )?;
        Ok(())
    }

    /// Push branch to remote with force push.
    pub fn push_branch(&self, branch: &str) -> Result<()> {
        info!("pushing branch {branch}");
        // setup callbacks for authentication
        let config = self.repo.config()?.snapshot()?;
        let user = config.get_str("user.name")?;
        let token = self.config.token.expose_secret().to_string();
        let callbacks = get_auth_callbacks(user.into(), token.clone());
        let mut push_opts = git2::PushOptions::default();
        push_opts.remote_callbacks(callbacks);

        let mut remote = self.repo.find_remote(DEFAULT_UPSTREAM_REMOTE)?;

        // + indicates "force" push
        let ref_spec = format!("+refs/heads/{branch}");
        remote.push(&[ref_spec], Some(&mut push_opts))?;

        Ok(())
    }

    /// Create annotated git tag pointing to specified commit.
    pub fn tag_commit(&self, tag: &str, commit_str: &str) -> Result<()> {
        let config = self.repo.config()?.snapshot()?;
        let user = config.get_str("user.name")?;
        let email = config.get_str("user.email")?;

        let oid = Oid::from_str(commit_str)?;
        let commit = self.repo.find_commit(oid)?;
        let tagger = git2::Signature::now(user, email)?;

        self.repo
            .tag(tag, commit.as_object(), &tagger, tag, false)?;

        Ok(())
    }

    /// Push git tag to remote repository.
    pub fn push_tag_to_default_branch(&self, tag: &str) -> Result<()> {
        // setup callbacks for authentication
        let config = self.repo.config()?.snapshot()?;
        let user = config.get_str("user.name")?;
        let token = self.config.token.expose_secret().to_string();
        let callbacks = get_auth_callbacks(user.into(), token.clone());
        let mut push_opts = git2::PushOptions::default();
        push_opts.remote_callbacks(callbacks);

        let mut remote = self.repo.find_remote(DEFAULT_UPSTREAM_REMOTE)?;

        let ref_spec = format!("refs/tags/{tag}");
        remote.push(&[ref_spec], Some(&mut push_opts))?;

        Ok(())
    }

    /// Get repository working directory path.
    pub fn workdir(&self) -> Result<&Path> {
        self.repo
            .workdir()
            .ok_or_else(|| eyre!("Repository has no working directory"))
    }

    /// Get the repository's working directory as a string path.
    ///
    /// Returns the working directory path as a string slice, with a fallback
    /// to the current directory (".") if the working directory cannot be
    /// determined or converted to a valid UTF-8 string.
    ///
    ///
    /// # Fallback Behavior
    ///
    /// This method never fails, returning "." in cases where:
    /// - The repository has no working directory (bare repository)
    /// - The working directory path contains invalid UTF-8 characters
    pub fn workdir_as_str(&self) -> &str {
        if let Some(w) = self.repo.workdir()
            && let Some(p) = w.to_str()
        {
            return p;
        }

        "."
    }
}
