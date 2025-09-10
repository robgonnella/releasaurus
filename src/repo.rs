//! Git repository operations and management.
//!
//! This module provides a high-level interface for interacting with Git repositories,
//! specifically designed for release automation workflows. It handles:
//!
//! - Repository cloning and authentication
//! - Branch creation and management
//! - Commit operations and tagging
//! - Remote repository interactions
//! - Tag-based version history analysis
//!
//! # Key Features
//!
//! - **Shallow Cloning**: Clones repositories with limited depth to optimize performance
//! - **Multi-Platform Authentication**: Supports token-based authentication for GitHub, GitLab, and Gitea
//! - **Branch Management**: Create, switch, and push branches for release workflows
//! - **Tag Operations**: Create and push version tags with proper commit association
//! - **Starting Point Detection**: Find the latest tagged release for changelog generation
//!
//! # Authentication
//!
//! Authentication is handled through Git credentials using username/token pairs.
//! Tokens should have appropriate repository permissions for the intended operations.
//!
//! # Usage
//!
//! ```rust,ignore
//! use std::path::Path;
//! use crate::forge::config::RemoteConfig;
//!
//! let repo = Repository::new(Path::new("./local-repo"), remote_config)?;
//! repo.create_branch("release-prep")?;
//! repo.switch_branch("release-prep")?;
//! // Make changes...
//! repo.add_all()?;
//! repo.commit("Prepare release v1.0.0")?;
//! repo.push_branch("release-prep")?;
//! ```
use color_eyre::eyre::eyre;
use git2::{Oid, RemoteCallbacks};
use log::*;
use regex::Regex;
use reqwest::Url;
use secrecy::ExposeSecret;
use std::path::Path;

use crate::{forge::config::RemoteConfig, result::Result};

/// Default name for the upstream remote repository.
///
/// After cloning, the original "origin" remote is renamed to this value to maintain
/// consistency with typical fork-based workflows where "origin" refers to the user's
/// fork and "upstream" refers to the main repository.
const DEFAULT_UPSTREAM_REMOTE: &str = "upstream";

/// Represents a tagged commit and its parent for changelog generation.
///
/// This structure captures the information needed to generate changelogs by providing
/// both the tagged commit (end point) and its parent (start point) for analyzing
/// the range of commits included in a release.
///
/// # Fields
///
/// * `tagged_commit` - The commit hash that was tagged for a release
/// * `tagged_parent` - The parent commit hash, used as the starting point for changelog generation
///
/// # Usage
///
/// Used by the analyzer to determine the range of commits to include when generating
/// changelogs for a new release, ensuring all commits since the last tagged release
/// are properly documented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartingPoint {
    /// The commit hash of the tagged release.
    pub tagged_commit: String,
    /// The parent commit hash of the tagged release, used as the changelog starting point.
    pub tagged_parent: String,
}

/// High-level Git repository interface for release automation.
///
/// This structure wraps the `git2::Repository` with additional functionality
/// specific to release workflows, including authentication, branch management,
/// and tag operations.
///
/// # Authentication
///
/// The repository uses the configured remote credentials for all operations
/// that require network access (clone, push, etc.). Credentials are sourced
/// from the `RemoteConfig` and combined with local Git configuration.
///
/// # Working Directory
///
/// All operations are performed in the repository's working directory, which
/// is established when the repository is cloned or opened.
pub struct Repository {
    /// The default branch name of the remote repository (e.g., "main" or "master").
    pub default_branch: String,
    /// Remote repository configuration including authentication details.
    config: RemoteConfig,
    /// The underlying git2 repository instance.
    repo: git2::Repository,
}

/// Create Git authentication callbacks for username/token authentication.
///
/// This function sets up the authentication mechanism used for all Git operations
/// that require network access. It uses plaintext username/token authentication,
/// which is suitable for HTTPS-based Git operations with personal access tokens.
///
/// # Arguments
///
/// * `user` - The username for authentication (typically from Git config)
/// * `token` - The access token or password for authentication
///
/// # Returns
///
/// * `RemoteCallbacks` - Configured callbacks for Git remote operations
///
/// # Security
///
/// The token is passed as plaintext, which is appropriate for HTTPS connections
/// where the transport layer provides encryption. The token should be treated
/// as sensitive and not logged or exposed in error messages.
fn get_auth_callbacks<'r>(user: String, token: String) -> RemoteCallbacks<'r> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(move |_url, _username, _allowed| {
        git2::Cred::userpass_plaintext(&user, &token)
    });
    callbacks
}

impl Repository {
    /// Create a new repository instance by cloning from a remote.
    ///
    /// This method performs a shallow clone of the remote repository with a maximum
    /// depth of 250 commits to optimize performance. The clone is authenticated using
    /// the provided remote configuration and local Git settings.
    ///
    /// # Arguments
    ///
    /// * `local_path` - Local filesystem path where the repository should be cloned
    /// * `config` - Remote repository configuration including URL and authentication
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - New repository instance or error if cloning fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The remote URL is invalid or inaccessible
    /// - Authentication fails
    /// - Local Git configuration is missing required fields (user.name)
    /// - The local path is invalid or not writable
    /// - Network connectivity issues prevent cloning
    ///
    /// # Shallow Clone Behavior
    ///
    /// The method uses a shallow clone with depth 250 to balance performance with
    /// functionality. This works best with repositories that maintain linear commit
    /// histories. For repositories with complex branching, some historical context
    /// may be limited.
    ///
    /// # Remote Naming
    ///
    /// After cloning, the default "origin" remote is renamed to "upstream" to
    /// follow conventional fork-based workflow naming.
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

        Ok(Self {
            repo,
            config,
            default_branch,
        })
    }

    /// Find the latest tagged commit matching the given prefix.
    ///
    /// This method searches through all Git tags in the repository to find the most
    /// recent tag that matches the specified prefix. It returns information about
    /// both the tagged commit and its parent, which is essential for changelog
    /// generation that needs to analyze commits since the last release.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Tag prefix to match (e.g., "v" for "v1.0.0" tags, or "" for any tag)
    ///
    /// # Returns
    ///
    /// * `Result<Option<StartingPoint>>` - The latest matching tag information, or None if no tags match
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The regex pattern for the prefix is invalid
    /// - Git references cannot be accessed
    /// - Tag commit resolution fails
    ///
    /// # Tag Matching
    ///
    /// Tags are matched using a regex pattern `^{prefix}` where the prefix is treated
    /// as a literal string. This allows for flexible tag naming schemes:
    ///
    /// - `""` matches any tag
    /// - `"v"` matches tags like "v1.0.0", "v2.1.0-beta"
    /// - `"api-v"` matches tags like "api-v1.0.0"
    ///
    /// # Usage
    ///
    /// ```rust,ignore
    /// // Find latest version tag
    /// let starting_point = repo.get_latest_tagged_starting_point("v")?;
    ///
    /// // Find latest API package tag
    /// let starting_point = repo.get_latest_tagged_starting_point("api-v")?;
    /// ```
    pub fn get_latest_tagged_starting_point(
        &self,
        prefix: &str,
    ) -> Result<Option<StartingPoint>> {
        let regex_prefix = format!(r"^{}", prefix);
        let tag_regex = Regex::new(&regex_prefix)?;
        let references = self
            .repo
            .references()?
            .filter_map(|r| r.ok())
            .collect::<Vec<git2::Reference>>();

        // Iterate through all references in the repository in reverse and stop
        // at first that matches prefix
        for reference in references.into_iter().rev() {
            // Check if the reference is a tag with desired prefix
            if reference.is_tag()
                && let Some(name) = reference.name()
                && let Some(stripped) = name.strip_prefix("refs/tags/")
                && tag_regex.is_match(stripped)
            {
                let commit = reference.peel_to_commit()?;

                // return the commit and the parent of the tagged commit so the
                // commit range can use the parent to get a full release in the
                // event no new commits have been added
                if let Ok(parent) = commit.parent(0) {
                    return Ok(Some(StartingPoint {
                        tagged_commit: commit.id().to_string(),
                        tagged_parent: parent.id().to_string(),
                    }));
                }
            }
        }
        Ok(None)
    }

    /// Create a new branch from the current HEAD.
    ///
    /// Creates a new branch pointing to the same commit as the current HEAD.
    /// The branch is created locally and can be switched to using `switch_branch()`.
    ///
    /// # Arguments
    ///
    /// * `branch` - Name of the branch to create
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error if branch creation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A branch with the same name already exists
    /// - The repository is in a detached HEAD state
    /// - Git operations fail due to repository corruption
    ///
    /// # Force Creation
    ///
    /// This method uses force creation (`true` parameter), meaning it will
    /// overwrite existing branches with the same name. This is intentional
    /// for release workflows where branches may be recreated.
    pub fn create_branch(&self, branch: &str) -> Result<()> {
        info!("creating branch: {branch}");
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        self.repo.branch(branch, &commit, true)?;
        Ok(())
    }

    /// Switch to the specified branch and update the working directory.
    ///
    /// This performs a checkout operation to switch the working directory
    /// to the specified branch. Both the HEAD reference and working directory
    /// files are updated to match the target branch.
    ///
    /// # Arguments
    ///
    /// * `branch` - Name of the branch to switch to
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error if branch switching fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The specified branch does not exist
    /// - Working directory has uncommitted changes that would conflict
    /// - Git operations fail due to repository issues
    ///
    /// # Working Directory Changes
    ///
    /// This operation will modify files in the working directory to match
    /// the target branch. Any uncommitted changes may cause conflicts
    /// and prevent the branch switch from completing.
    pub fn switch_branch(&self, branch: &str) -> Result<()> {
        info!("switching to branch: {branch}");
        let ref_name = format!("refs/heads/{}", branch);
        let target_obj = self.repo.revparse_single(&ref_name)?;
        self.repo.checkout_tree(&target_obj, None)?;
        self.repo.set_head(&ref_name)?;
        Ok(())
    }

    /// Add all changed files to the Git index (staging area).
    ///
    /// This method stages all modified, new, and deleted files in the working
    /// directory for the next commit. It's equivalent to running `git add .`.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error if staging fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The Git index cannot be accessed or modified
    /// - File system permissions prevent reading changed files
    /// - The repository is in an invalid state
    ///
    /// # Staging Behavior
    ///
    /// - **Modified files**: Existing files with changes are staged
    /// - **New files**: Untracked files are added to the index
    /// - **Deleted files**: File deletions are staged for commit
    /// - **Ignored files**: Files matching .gitignore patterns are skipped
    ///
    /// # Usage in Release Workflows
    ///
    /// Typically used after version files and changelogs have been updated
    /// to prepare all changes for a release commit.
    pub fn add_all(&self) -> Result<()> {
        debug!("adding changed files to index");
        let mut index = self.repo.index()?;
        index.add_all(["."], git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    /// Create a new commit with the currently staged changes.
    ///
    /// Creates a commit using the files currently in the Git index (staging area)
    /// with the specified commit message. The commit author and committer are
    /// determined from the local Git configuration.
    ///
    /// # Arguments
    ///
    /// * `msg` - Commit message describing the changes
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error if commit creation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Git configuration is missing required fields (user.name, user.email)
    /// - The index is empty (no staged changes)
    /// - The repository is in an invalid state
    /// - Git operations fail
    ///
    /// # Git Configuration Requirements
    ///
    /// This method requires the following Git configuration values:
    /// - `user.name`: The name to use for the commit author/committer
    /// - `user.email`: The email address for the commit author/committer
    ///
    /// # Commit Structure
    ///
    /// The created commit:
    /// - Has the current HEAD as its parent (normal linear commit)
    /// - Uses the same signature for both author and committer
    /// - Updates the HEAD reference to point to the new commit
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

    /// Push a local branch to the remote repository.
    ///
    /// Pushes the specified local branch to the upstream remote repository
    /// using force push to ensure the remote branch is updated even if it
    /// has diverged from the local branch.
    ///
    /// # Arguments
    ///
    /// * `branch` - Name of the local branch to push
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error if push operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The specified branch does not exist locally
    /// - Authentication fails with the remote repository
    /// - Network connectivity issues prevent pushing
    /// - The remote repository rejects the push due to permissions
    ///
    /// # Force Push Behavior
    ///
    /// This method uses force push (`+` prefix in refspec) to ensure that
    /// the remote branch is updated to match the local branch exactly.
    /// This is appropriate for release workflow branches that may be
    /// recreated or rebased.
    ///
    /// # Authentication
    ///
    /// Uses the repository's configured authentication credentials
    /// (username/token) to authenticate with the remote repository.
    ///
    /// # Network Requirements
    ///
    /// Requires network access to the remote repository and appropriate
    /// permissions to push to the target branch.
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

    /// Create a Git tag pointing to the specified commit.
    ///
    /// Creates an annotated Git tag with the given name pointing to the
    /// specified commit. The tag includes tagger information from the
    /// local Git configuration and uses the tag name as the tag message.
    ///
    /// # Arguments
    ///
    /// * `tag` - Name of the tag to create (e.g., "v1.0.0")
    /// * `commit_str` - Commit hash (SHA) to tag
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error if tag creation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The commit hash is invalid or doesn't exist
    /// - A tag with the same name already exists
    /// - Git configuration is missing tagger information
    /// - Git operations fail
    ///
    /// # Tag Type
    ///
    /// Creates an annotated tag (not a lightweight tag) which includes:
    /// - Tagger name and email from Git configuration
    /// - Timestamp of tag creation
    /// - Tag message (same as tag name)
    ///
    /// # Git Configuration Requirements
    ///
    /// Requires the following Git configuration:
    /// - `user.name`: Name for the tagger field
    /// - `user.email`: Email for the tagger field
    ///
    /// # Usage in Release Workflows
    ///
    /// Typically called after a release commit has been created to
    /// mark the specific commit as a released version.
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

    /// Push a Git tag to the remote repository.
    ///
    /// Pushes the specified local tag to the upstream remote repository,
    /// making it available to other users and triggering any automated
    /// processes that monitor for new tags (like CI/CD release pipelines).
    ///
    /// # Arguments
    ///
    /// * `tag` - Name of the tag to push
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error if push operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The specified tag does not exist locally
    /// - Authentication fails with the remote repository
    /// - Network connectivity issues prevent pushing
    /// - The remote repository rejects the push due to permissions
    /// - A tag with the same name already exists remotely (unless force pushed)
    ///
    /// # Authentication
    ///
    /// Uses the repository's configured authentication credentials
    /// to authenticate with the remote repository. The credentials
    /// must have appropriate permissions to create tags.
    ///
    /// # Remote Repository Effects
    ///
    /// Successfully pushing a tag may trigger:
    /// - Automated release processes
    /// - CI/CD pipeline executions
    /// - Package publication workflows
    /// - Notification systems
    ///
    /// # Tag Naming
    ///
    /// The method name references "default_branch" for historical reasons,
    /// but it actually pushes the tag to the remote repository where it
    /// exists independently of any specific branch.
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

    /// Get the repository's working directory path.
    ///
    /// Returns a reference to the filesystem path of the repository's
    /// working directory where files can be read and modified.
    ///
    /// # Returns
    ///
    /// * `Result<&Path>` - Path to the working directory or error if unavailable
    ///
    /// # Errors
    ///
    /// Returns an error if the repository doesn't have a working directory,
    /// which can occur with bare repositories or in certain Git operations.
    ///
    /// # Usage
    ///
    /// Used by other modules to determine where to read and write files
    /// as part of the release process, such as version files and changelogs.
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
    /// # Returns
    ///
    /// * `&str` - Working directory path as string, or "." as fallback
    ///
    /// # Fallback Behavior
    ///
    /// This method never fails, returning "." in cases where:
    /// - The repository has no working directory (bare repository)
    /// - The working directory path contains invalid UTF-8 characters
    ///
    /// # Usage
    ///
    /// Useful for logging, display purposes, or when a string representation
    /// of the working directory is needed for external tools or APIs.
    pub fn workdir_as_str(&self) -> &str {
        if let Some(w) = self.repo.workdir()
            && let Some(p) = w.to_str()
        {
            return p;
        }

        "."
    }
}
