//! Local forge implementation for offline development and testing.
use async_trait::async_trait;
use color_eyre::eyre::{Context, OptionExt};
use git2::{
    BranchType, Commit as Git2Commit, Oid, RemoteCallbacks, Sort,
    StatusOptions, TreeWalkMode,
};
use regex::Regex;
use secrecy::{ExposeSecret, SecretString};
use std::{
    path::{self, Path, PathBuf},
    sync::Arc,
};
use tokio::{fs, sync::Mutex};
use url::Url;

use crate::{
    analyzer::release::Tag,
    config::{Config, DEFAULT_CONFIG_FILE},
    error::{ReleasaurusError, Result},
    forge::{
        config::RepoUrl,
        request::{
            Commit, CreateCommitRequest, CreatePrRequest,
            CreateReleaseBranchRequest, FileChange, FileUpdateType,
            ForgeCommit, GetFileContentRequest, GetPrRequest, PrLabelsRequest,
            PullRequest, ReleaseByTagResponse, UpdatePrRequest,
        },
        traits::Forge,
    },
};

/// Create Git authentication callbacks for token-based HTTPS auth.
/// Uses "git" as a fixed username placeholder — all supported forges
/// (GitHub, GitLab, Gitea) authenticate solely via the token/password
/// and ignore the username field.
fn get_auth_callbacks<'r>(token: String) -> RemoteCallbacks<'r> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(move |_url, _username, _allowed| {
        git2::Cred::userpass_plaintext("git", &token)
    });
    callbacks
}

pub struct Remote {
    pub forge: Arc<dyn Forge>,
    pub token: SecretString,
    pub url: RepoUrl,
}

/// LocalRepo forge implementation using git2 for local repository operations.
pub struct LocalRepo {
    repo_path: PathBuf,
    repo_name: String,
    repo: Arc<Mutex<git2::Repository>>,
    default_branch: String,
    link_base_url: Url,
    remote: Option<Remote>,
    // only used in testing
    push_targets_disabled: bool,
}

impl LocalRepo {
    pub fn new(repo_path: &Path, remote: Option<Remote>) -> Result<Self> {
        let repo_str = repo_path.to_string_lossy();
        let abs_repo_path = path::absolute(repo_path)?;
        log::debug!(
            "LocalRepo::new: repo_path={}, abs_repo_path={}",
            repo_str,
            abs_repo_path.display()
        );

        if !abs_repo_path.exists() {
            return Err(ReleasaurusError::forge(format!(
                "Invalid path for local forge: {repo_str} does not exist"
            )));
        }

        if !abs_repo_path.is_dir() {
            return Err(ReleasaurusError::forge(format!(
                "Invalid path for local forge: {repo_str} is not a directory"
            )));
        }

        let mut link_base_url =
            Url::from_file_path(&abs_repo_path).map_err(|_| {
                ReleasaurusError::forge(format!(
                    "Unable to create file URL from path: {}",
                    abs_repo_path.display()
                ))
            })?;

        // Ensure trailing slash so Url::join() appends rather than replaces
        link_base_url.set_path(&format!("{}/", link_base_url.path()));

        let repo_name = abs_repo_path
            .file_name()
            .ok_or(ReleasaurusError::forge(
                "unable to determine repository directory name from path",
            ))?
            .to_string_lossy()
            .to_string();

        log::debug!("LocalRepo::new: opening repository at {}", repo_str);
        let repo = git2::Repository::open(repo_path)?;

        let head = repo.head()?;

        let default_branch = head
            .shorthand()
            .ok_or_eyre("unable to get current branch for local repo")?
            .to_string();

        drop(head);

        Ok(Self {
            repo_name,
            repo_path: repo_path.to_path_buf(),
            repo: Arc::new(Mutex::new(repo)),
            default_branch,
            link_base_url,
            remote,
            push_targets_disabled: false,
        })
    }

    #[cfg(test)]
    pub(crate) fn disable_push_targets(&mut self) {
        self.push_targets_disabled = true
    }

    async fn get_current_branch(&self) -> Result<String> {
        let repo = self.repo.lock().await;
        let head = repo.head()?;
        let current_branch = head
            .shorthand()
            .ok_or(ReleasaurusError::git_other(
                "unable to get current branch for local repo",
            ))?
            .to_string();
        Ok(current_branch)
    }

    /// Create new branch from provided base_ref. If the new branch already
    /// exists it is force overwritten to start from the base_ref
    async fn create_branch(&self, branch: &str, base_ref: &str) -> Result<()> {
        let repo = self.repo.lock().await;
        log::debug!("finding base ref: {base_ref}");
        let base_branch = repo.find_branch(base_ref, BranchType::Local)?;
        let base_branch = base_branch.get();
        let commit = base_branch.peel_to_commit()?;
        log::debug!("base_branch={base_ref} commit={}", commit.id());
        log::info!("creating branch: {branch}");
        repo.branch(branch, &commit, true)?;
        Ok(())
    }

    /// Switch to branch and update working directory.
    async fn switch_branch(&self, branch: &str) -> Result<()> {
        log::info!("switching to branch: {branch}");
        let repo = self.repo.lock().await;
        let ref_name = format!("refs/heads/{}", branch);
        log::debug!("switch_branch: resolving ref {ref_name}");
        let target_obj = repo.revparse_single(&ref_name)?;
        log::debug!("switch_branch: checking out tree for {ref_name}");
        repo.checkout_tree(&target_obj, None)?;
        log::debug!("switch_branch: setting HEAD to {ref_name}");
        repo.set_head(&ref_name)?;
        log::debug!("switch_branch: done");
        Ok(())
    }

    /// Add file path to git index (equivalent to `git add <file_path>`).
    /// Accepts both absolute paths and paths relative to the workdir.
    async fn stage_file(&self, path: &Path) -> Result<()> {
        log::info!("adding file path to index: {}", path.display());
        let repo = self.repo.lock().await;
        let mut index = repo.index()?;
        // git2 requires a path relative to the repo workdir.
        // Strip the repo root prefix for absolute paths, or the
        // leading "./" for explicitly relative ones.
        let relative = if path.is_absolute() {
            path.strip_prefix(&self.repo_path).unwrap_or(path)
        } else {
            path.strip_prefix("./").unwrap_or(path)
        };
        index.add_path(relative)?;
        index.write()?;
        Ok(())
    }

    /// Create commit with staged changes and specified message.
    async fn local_commit(
        &self,
        msg: &str,
        file_changes: &[FileChange],
    ) -> Result<Commit> {
        log::debug!(
            "local_commit: repo_path={}, file_changes count={}",
            self.repo_path.display(),
            file_changes.len()
        );
        for change in file_changes {
            let full_path = self.repo_path.join(&change.path);
            log::debug!(
                "local_commit: processing file change: path={}, full_path={}, update_type={:?}",
                change.path,
                full_path.display(),
                change.update_type
            );
            let mut content = change.content.clone();
            if change.update_type == FileUpdateType::Prepend {
                if let Ok(existing_content) =
                    fs::read_to_string(&full_path).await
                {
                    log::debug!(
                        "local_commit: read existing content from {} ({} bytes)",
                        full_path.display(),
                        existing_content.len()
                    );
                    content = format!("{content}{existing_content}");
                } else {
                    log::debug!(
                        "local_commit: no existing file at {}, creating new",
                        full_path.display()
                    );
                }
            }
            if let Some(parent) = full_path.parent() {
                log::debug!(
                    "local_commit: parent dir {} exists={}",
                    parent.display(),
                    parent.exists()
                );
                fs::create_dir_all(parent).await?;
            }
            fs::write(&full_path, content).await?;
            self.stage_file(&full_path).await?;
        }

        log::debug!("committing changes with msg: {msg}");
        let repo = self.repo.lock().await;

        let mut options = StatusOptions::new();
        let statuses = repo.statuses(Some(&mut options))?;

        // If the list of statuses is empty, there are no changes to be committed
        if !statuses.is_empty() {
            let config = repo.config()?.snapshot()?;
            let user = config.get_str("user.name")?;
            let email = config.get_str("user.email")?;
            log::debug!("using committer: user: {user}, email: {email}");
            let mut index = repo.index()?;
            let oid = index.write_tree()?;
            let tree = repo.find_tree(oid)?;
            let parent_commit = repo.head()?.peel_to_commit()?;
            let committer = git2::Signature::now(user, email)?;
            let oid = repo.commit(
                Some("HEAD"),
                &committer,
                &committer,
                msg,
                &tree,
                &[&parent_commit],
            )?;

            Ok(Commit {
                sha: oid.to_string(),
            })
        } else {
            Err(ReleasaurusError::git_other("nothing to commit"))
        }
    }

    /// Push branch to remote with option to force push.
    async fn push_branch(&self, branch: &str, force: bool) -> Result<()> {
        if let Some(remote) = self.remote.as_ref() {
            log::info!("pushing branch {branch}");
            let repo = self.repo.lock().await;
            let token = remote.token.expose_secret().to_string();
            let callbacks = get_auth_callbacks(token);
            let mut push_opts = git2::PushOptions::default();
            push_opts.remote_callbacks(callbacks);

            let mut git_remote =
                repo.remote_anonymous(&remote.url.to_string())?;

            let mut ref_spec = format!("refs/heads/{branch}");

            if force {
                // + indicates "force" push
                ref_spec = format!("+{ref_spec}");
            }

            if !self.push_targets_disabled {
                git_remote.push(&[ref_spec], Some(&mut push_opts))?;
            }
        } else {
            log::warn!("no remote configured: skipping branch push")
        }

        Ok(())
    }

    /// Create annotated git tag pointing to specified commit.
    async fn local_tag_commit(&self, tag: &str, sha: &str) -> Result<()> {
        let repo = self.repo.lock().await;
        let config = repo.config()?.snapshot()?;
        let user = config.get_str("user.name")?;
        let email = config.get_str("user.email")?;

        let oid = Oid::from_str(sha)?;
        let commit = repo.find_commit(oid)?;
        let tagger = git2::Signature::now(user, email)?;

        repo.tag(tag, commit.as_object(), &tagger, tag, false)?;

        Ok(())
    }

    /// Push git tag to remote repository.
    async fn push_tag(&self, tag: &str) -> Result<()> {
        if let Some(remote) = self.remote.as_ref() {
            let repo = self.repo.lock().await;
            let token = remote.token.expose_secret().to_string();
            let callbacks = get_auth_callbacks(token);
            let mut push_opts = git2::PushOptions::default();
            push_opts.remote_callbacks(callbacks);
            let mut git_remote =
                repo.remote_anonymous(&remote.url.to_string())?;
            let ref_spec = format!("refs/tags/{tag}");
            if !self.push_targets_disabled {
                git_remote.push(&[ref_spec], Some(&mut push_opts))?;
            }
        } else {
            log::warn!("no remote configured: skipping tag push")
        }

        Ok(())
    }
}

#[async_trait]
impl Forge for LocalRepo {
    fn repo_name(&self) -> String {
        if let Some(remote) = self.remote.as_ref() {
            remote.forge.repo_name()
        } else {
            self.repo_name.clone()
        }
    }

    fn default_branch(&self) -> String {
        if let Some(remote) = self.remote.as_ref() {
            remote.forge.default_branch()
        } else {
            self.default_branch.clone()
        }
    }

    fn release_link_base_url(&self) -> Url {
        if let Some(remote) = self.remote.as_ref() {
            remote.forge.release_link_base_url()
        } else {
            self.link_base_url.clone()
        }
    }

    fn compare_link_base_url(&self) -> Url {
        if let Some(remote) = self.remote.as_ref() {
            remote.forge.compare_link_base_url()
        } else {
            self.link_base_url.clone()
        }
    }

    async fn get_file_content(
        &self,
        req: GetFileContentRequest,
    ) -> Result<Option<String>> {
        let full_path = Path::new(&self.repo_path).join(&req.path);
        if !full_path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(full_path).await?;
        Ok(Some(content))
    }

    async fn load_config(&self, branch: Option<String>) -> Result<Config> {
        if let Some(content) = self
            .get_file_content(GetFileContentRequest {
                branch,
                path: DEFAULT_CONFIG_FILE.into(),
            })
            .await?
        {
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            log::info!("repository configuration not found: using default");
            Ok(Config::default())
        }
    }

    async fn get_release_by_tag(
        &self,
        tag: &str,
    ) -> Result<ReleaseByTagResponse> {
        if let Some(remote) = self.remote.as_ref() {
            remote.forge.get_release_by_tag(tag).await
        } else {
            Err(ReleasaurusError::forge("not implemented for local forge"))
        }
    }

    async fn get_latest_tags_for_prefix(
        &self,
        prefix: &str,
        branch: &str,
    ) -> Result<Vec<Tag>> {
        let regex_prefix = format!(r"^{}", prefix);
        let tag_prefix_regex = Regex::new(&regex_prefix)?;

        let repo = self.repo.lock().await;

        let references = repo
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
                    timestamp: Some(commit.time().seconds()),
                };

                commits.push((commit, tag));
            }
        }

        if commits.is_empty() {
            return Ok(vec![]);
        }

        let branch_head_oid = repo
            .find_branch(branch, BranchType::Local)?
            .get()
            .peel_to_commit()?
            .id();

        // Keep only tags that are ancestors of the branch head.
        // graph_descendant_of(a, b) returns true if a is a descendant of b,
        // so graph_descendant_of(branch_head, tag) means tag is an ancestor
        // of branch_head. We also include the case where the tag IS the
        // branch head (e.g., immediately after a release is tagged).
        commits.retain(|(commit, _)| {
            commit.id() == branch_head_oid
                || repo
                    .graph_descendant_of(branch_head_oid, commit.id())
                    .unwrap_or(false)
        });

        Ok(commits.into_iter().map(|(_, tag)| tag).collect())
    }

    async fn get_commits(
        &self,
        _branch: Option<String>,
        sha: Option<String>,
    ) -> Result<Vec<ForgeCommit>> {
        let repo = self.repo.lock().await;

        let mut revwalk = repo.revwalk()?;

        revwalk.set_sorting(Sort::TIME)?;

        if let Some(sha) = sha {
            revwalk.push_range(&format!("{sha}..HEAD"))?;
        } else {
            revwalk.push_head()?;
        }

        let commits: Vec<Git2Commit> = revwalk
            .filter_map(|id| id.ok())
            .filter_map(|id| repo.find_commit(id).ok())
            .collect();

        let mut forge_commits = vec![];

        for commit in commits.iter() {
            // shamelessly borrowed from git-cliff-core
            let changed_files: Result<Vec<PathBuf>> = {
                let mut changes = vec![];

                if let Ok(prev_commit) = commit.parent(0) {
                    // Compare the current commit with the previous commit to
                    // get the changed files.
                    // libgit2 does not provide a way to get the changed files
                    // directly, so the full diff is calculated here.
                    if let Ok(diff) = repo.diff_tree_to_tree(
                        commit.tree().ok().as_ref(),
                        prev_commit.tree().ok().as_ref(),
                        None,
                    ) {
                        changes.extend(diff.deltas().filter_map(|delta| {
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

                          changes.push(entry_path.into());

                          0
                      })
                      .wrap_err(
                          "failed to get the changed files of the first commit",
                      )?;
                    }
                }

                Ok(changes)
            };

            let changed_files = changed_files?;

            let files = changed_files
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<String>>();

            forge_commits.push(ForgeCommit {
                author_email: commit.author().email().unwrap_or("").to_string(),
                author_name: commit.author().name().unwrap_or("").to_string(),
                files,
                id: commit.id().to_string(),
                link: "".into(),
                merge_commit: commit.parent_count() > 1,
                message: commit.message().unwrap_or("").to_string(),
                short_id: commit
                    .id()
                    .to_string()
                    .split("")
                    .take(8)
                    .collect::<Vec<&str>>()
                    .join(""),
                timestamp: commit.time().seconds(),
            });
        }

        Ok(forge_commits)
    }

    async fn create_release_branch(
        &self,
        req: CreateReleaseBranchRequest,
    ) -> Result<Commit> {
        if self.remote.is_some() {
            let current_branch = self.get_current_branch().await?;
            self.create_branch(&req.release_branch, &req.base_branch)
                .await?;
            self.switch_branch(&req.release_branch).await?;
            let commit =
                self.local_commit(&req.message, &req.file_changes).await?;
            self.push_branch(&req.release_branch, true).await?;
            self.switch_branch(&current_branch).await?;
            Ok(commit)
        } else {
            log::warn!("local_mode: would create branch: req: {:#?}", req);
            Ok(Commit { sha: "None".into() })
        }
    }

    async fn create_commit(&self, req: CreateCommitRequest) -> Result<Commit> {
        if self.remote.is_some() {
            let commit =
                self.local_commit(&req.message, &req.file_changes).await?;
            self.push_branch(&req.target_branch, false).await?;
            Ok(commit)
        } else {
            log::warn!("local_mode: would create commit: req: {:#?}", req);
            Ok(Commit { sha: "None".into() })
        }
    }

    async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()> {
        if self.remote.is_some() {
            self.local_tag_commit(tag_name, sha).await?;
            self.push_tag(tag_name).await?;
            Ok(())
        } else {
            log::warn!(
                "local_mode: would tag commit: \
                 tag_name: {tag_name}, sha: {sha}"
            );
            Ok(())
        }
    }

    async fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        if let Some(remote) = self.remote.as_ref() {
            remote.forge.get_open_release_pr(req).await
        } else {
            log::warn!(
                "local_mode: would request open release pr: req: {:#?}",
                req
            );
            Ok(None)
        }
    }

    async fn get_merged_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        if let Some(remote) = self.remote.as_ref() {
            remote.forge.get_merged_release_pr(req).await
        } else {
            log::warn!(
                "local_mode: would request merged release pr: req: {:#?}",
                req
            );
            Ok(None)
        }
    }

    async fn create_pr(&self, req: CreatePrRequest) -> Result<PullRequest> {
        if let Some(remote) = self.remote.as_ref() {
            remote.forge.create_pr(req).await
        } else {
            log::warn!("local_mode: would create release pr: req: {:#?}", req);
            Ok(PullRequest {
                number: 0,
                sha: "None".into(),
                body: req.body,
            })
        }
    }

    async fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        if let Some(remote) = self.remote.as_ref() {
            remote.forge.update_pr(req).await
        } else {
            log::warn!("local_mode: would update release pr: req: {:#?}", req);
            Ok(())
        }
    }

    async fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        if let Some(remote) = self.remote.as_ref() {
            remote.forge.replace_pr_labels(req).await
        } else {
            log::warn!("local_mode: would replace pr labels: req: {:#?}", req);
            Ok(())
        }
    }

    async fn create_release(
        &self,
        tag: &str,
        sha: &str,
        notes: &str,
    ) -> Result<()> {
        if let Some(remote) = self.remote.as_ref() {
            remote.forge.create_release(tag, sha, notes).await
        } else {
            log::warn!(
                "local_mode: would create release: tag: {tag}, sha: {sha}, notes: {notes}"
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::{
        config::Scheme,
        request::{FileChange, FileUpdateType},
        traits::MockForge,
    };
    use tempfile::TempDir;

    fn create_branch(
        repo: &git2::Repository,
        branch: &str,
        base_ref: &str,
    ) -> Result<()> {
        let base_branch = repo.find_branch(base_ref, BranchType::Local)?;
        let base_reference = base_branch.get();
        let commit = base_reference.peel_to_commit()?;
        repo.branch(branch, &commit, true)?;
        Ok(())
    }

    fn switch_branch(repo: &git2::Repository, branch: &str) -> Result<()> {
        let ref_name = format!("refs/heads/{}", branch);
        let target_obj = repo.revparse_single(&ref_name)?;
        repo.checkout_tree(&target_obj, None)?;
        repo.set_head(&ref_name)?;
        Ok(())
    }

    /// Creates a commit on whatever HEAD currently points to.
    /// For the very first commit (unborn branch) pass no parent —
    /// git2 handles that transparently via HEAD resolution.
    fn add_commit(repo: &git2::Repository, message: &str) -> git2::Oid {
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let mut index = repo.index().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<_> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }

    fn configure_git_user(repo: &git2::Repository) {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }

    fn tag_oid(repo: &git2::Repository, name: &str, oid: git2::Oid) {
        let obj = repo.find_object(oid, None).unwrap();
        repo.tag_lightweight(name, &obj, false).unwrap();
    }

    fn current_branch_name(repo: &git2::Repository) -> String {
        repo.head().unwrap().shorthand().unwrap().to_string()
    }

    /// Regression test: a tag that points to the exact same commit
    /// as the branch head must be returned. Previously,
    /// `graph_descendant_of` (a strict check) would return `false`
    /// here and the tag was wrongly filtered out.
    #[tokio::test]
    async fn tag_at_branch_head_is_found() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let oid = add_commit(&repo, "initial commit");
        tag_oid(&repo, "v1.0.0", oid);
        let branch = current_branch_name(&repo);

        let forge = LocalRepo::new(dir.path(), None).unwrap();
        let mut result = forge
            .get_latest_tags_for_prefix("v", &branch)
            .await
            .unwrap();
        result.sort_by(|a, b| b.semver.cmp(&a.semver));

        assert!(!result.is_empty(), "tag at branch head should be found");
        assert_eq!(result[0].name, "v1.0.0");
    }

    /// `create_branch` must create a branch pointing to current HEAD.
    #[tokio::test]
    async fn create_branch_from_base_branch() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let oid = add_commit(&repo, "initial commit");
        let base_branch = current_branch_name(&repo);

        let forge = LocalRepo::new(dir.path(), None).unwrap();
        forge.create_branch("release", &base_branch).await.unwrap();

        let repo = git2::Repository::open(dir.path()).unwrap();
        let branch = repo
            .find_branch("release", git2::BranchType::Local)
            .unwrap();
        let branch_oid = branch.get().peel_to_commit().unwrap().id();
        assert_eq!(branch_oid, oid);
    }

    /// `create_branch` must force-overwrite an existing branch,
    /// moving it to the current HEAD.
    #[tokio::test]
    async fn create_branch_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let base_commit_oid = add_commit(&repo, "initial commit");
        let base_branch = current_branch_name(&repo);

        create_branch(&repo, "test1", &base_branch).unwrap();
        switch_branch(&repo, "test1").unwrap();
        add_commit(&repo, "test1 commit");

        create_branch(&repo, "test2", &base_branch).unwrap();
        switch_branch(&repo, "test2").unwrap();
        add_commit(&repo, "test2 commit");

        switch_branch(&repo, &base_branch).unwrap();

        let forge = LocalRepo::new(dir.path(), None).unwrap();
        // Both should be overwritten with main.
        forge.create_branch("test1", &base_branch).await.unwrap();
        forge.create_branch("test2", &base_branch).await.unwrap();

        let repo = git2::Repository::open(dir.path()).unwrap();
        let test1_branch =
            repo.find_branch("test1", git2::BranchType::Local).unwrap();
        let test2_branch =
            repo.find_branch("test2", git2::BranchType::Local).unwrap();
        let test1_oid = test1_branch.get().peel_to_commit().unwrap().id();
        let test2_oid = test2_branch.get().peel_to_commit().unwrap().id();
        assert_eq!(test1_oid, base_commit_oid);
        assert_eq!(test2_oid, base_commit_oid);
    }

    /// `switch_branch` must move HEAD to the target branch.
    #[tokio::test]
    async fn switch_branch_updates_head() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        add_commit(&repo, "initial commit");
        let base_branch = current_branch_name(&repo);

        let forge = LocalRepo::new(dir.path(), None).unwrap();
        forge.create_branch("release", &base_branch).await.unwrap();
        forge.switch_branch("release").await.unwrap();

        let repo = git2::Repository::open(dir.path()).unwrap();
        let head = current_branch_name(&repo);
        assert_eq!(head, "release");
    }

    /// `stage_file` must add the file to the git index.
    #[tokio::test]
    async fn stage_file_adds_to_index() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        add_commit(&repo, "initial commit");

        // Write a new file to the working directory.
        let file_path = dir.path().join("staged.txt");
        std::fs::write(&file_path, "hello").unwrap();

        let forge = LocalRepo::new(dir.path(), None).unwrap();
        forge
            .stage_file(std::path::Path::new("staged.txt"))
            .await
            .unwrap();

        let repo = git2::Repository::open(dir.path()).unwrap();
        let index = repo.index().unwrap();
        let entry = index.get_path(std::path::Path::new("staged.txt"), 0);
        assert!(entry.is_some(), "staged.txt should be in the index");
    }

    /// `local_commit` must create a commit with the given message
    /// and write the supplied file content to disk.
    #[tokio::test]
    async fn local_commit_creates_commit_with_message() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        configure_git_user(&repo);
        add_commit(&repo, "initial commit");

        let forge = LocalRepo::new(dir.path(), None).unwrap();
        let change = FileChange {
            path: dir.path().join("version.txt").to_string_lossy().to_string(),
            content: "1.2.3".to_string(),
            update_type: FileUpdateType::Replace,
        };

        let commit = forge
            .local_commit("chore: bump version", &[change])
            .await
            .unwrap();

        assert!(!commit.sha.is_empty());

        let repo = git2::Repository::open(dir.path()).unwrap();
        let msg = repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .message()
            .unwrap()
            .to_string();
        assert_eq!(msg, "chore: bump version");

        let written =
            std::fs::read_to_string(dir.path().join("version.txt")).unwrap();
        assert_eq!(written, "1.2.3");
    }

    /// `local_commit` with `Prepend` update type must prepend the new
    /// content in front of the existing file content.
    #[tokio::test]
    async fn local_commit_with_prepend_updates_content() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        configure_git_user(&repo);

        // Create the initial file and commit it.
        let file_path = dir.path().join("CHANGELOG.md");
        std::fs::write(&file_path, "existing\n").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new("CHANGELOG.md"))
            .unwrap();
        index.write().unwrap();
        add_commit(&repo, "initial commit");

        let forge = LocalRepo::new(dir.path(), None).unwrap();
        let change = FileChange {
            path: file_path.to_string_lossy().to_string(),
            content: "new\n".to_string(),
            update_type: FileUpdateType::Prepend,
        };

        forge
            .local_commit("chore: update changelog", &[change])
            .await
            .unwrap();

        let written = std::fs::read_to_string(&file_path).unwrap();
        assert!(
            written.starts_with("new\n"),
            "prepended content should come first"
        );
        assert!(
            written.contains("existing\n"),
            "existing content should be preserved"
        );
    }

    /// `local_commit` with `Prepend` on a file that does not yet exist
    /// must create the file with just the new content (first release).
    #[tokio::test]
    async fn local_commit_with_prepend_creates_missing_file() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        configure_git_user(&repo);
        add_commit(&repo, "initial commit");

        let forge = LocalRepo::new(dir.path(), None).unwrap();
        let change = FileChange {
            path: "CHANGELOG.md".to_string(),
            content: "# 1.0.0\n\n- first release\n".to_string(),
            update_type: FileUpdateType::Prepend,
        };

        forge
            .local_commit("chore: update changelog", &[change])
            .await
            .unwrap();

        let written =
            std::fs::read_to_string(dir.path().join("CHANGELOG.md")).unwrap();
        assert_eq!(written, "# 1.0.0\n\n- first release\n");
    }

    /// `local_commit` must resolve relative `FileChange` paths against
    /// `repo_path` so that file I/O works regardless of the process CWD.
    #[tokio::test]
    async fn local_commit_resolves_relative_paths_against_repo_path() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        configure_git_user(&repo);
        add_commit(&repo, "initial commit");

        // Create a sub-directory to mimic a monorepo package path.
        let sub_dir = dir.path().join("packages").join("ui");
        std::fs::create_dir_all(&sub_dir).unwrap();

        let forge = LocalRepo::new(dir.path(), None).unwrap();
        let change = FileChange {
            path: "packages/ui/version.txt".to_string(),
            content: "1.0.0".to_string(),
            update_type: FileUpdateType::Replace,
        };

        forge
            .local_commit("chore: bump version", &[change])
            .await
            .unwrap();

        let written =
            std::fs::read_to_string(sub_dir.join("version.txt")).unwrap();
        assert_eq!(written, "1.0.0");
    }

    /// `local_commit` must return an error when there are no staged
    /// changes to commit.
    #[tokio::test]
    async fn local_commit_returns_error_when_nothing_to_commit() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        configure_git_user(&repo);
        add_commit(&repo, "initial commit");

        let forge = LocalRepo::new(dir.path(), None).unwrap();
        let result = forge.local_commit("chore: empty", &[]).await;
        assert!(
            result.is_err(),
            "should error when there is nothing to commit"
        );
    }

    /// `local_tag_commit` must create an annotated tag pointing to
    /// the specified commit OID.
    #[tokio::test]
    async fn local_tag_commit_creates_annotated_tag() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        configure_git_user(&repo);
        let oid = add_commit(&repo, "initial commit");

        let forge = LocalRepo::new(dir.path(), None).unwrap();
        forge
            .local_tag_commit("v1.0.0", &oid.to_string())
            .await
            .unwrap();

        let repo = git2::Repository::open(dir.path()).unwrap();
        // find_tag resolves annotated tags only (not lightweight).
        let tag_ref = repo.find_reference("refs/tags/v1.0.0").unwrap();
        let tag = tag_ref.peel_to_tag().unwrap();
        assert_eq!(tag.name().unwrap(), "v1.0.0");
        assert_eq!(tag.target_id(), oid);
    }

    /// `push_branch` must succeed (no-op) when no remote is
    /// configured.
    #[tokio::test]
    async fn push_branch_without_remote_is_noop() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        add_commit(&repo, "initial commit");
        let base_branch = current_branch_name(&repo);

        let forge = LocalRepo::new(dir.path(), None).unwrap();
        forge.push_branch(&base_branch, false).await.unwrap();
    }

    /// `push_tag` must succeed (no-op) when no remote is configured.
    #[tokio::test]
    async fn push_tag_without_remote_is_noop() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let oid = add_commit(&repo, "initial commit");
        tag_oid(&repo, "v1.0.0", oid);

        let forge = LocalRepo::new(dir.path(), None).unwrap();
        forge.push_tag("v1.0.0").await.unwrap();
    }

    /// A tag that lives on a divergent branch must NOT be returned
    /// when querying a branch that does not have that commit in its
    /// history.
    #[tokio::test]
    async fn tag_on_divergent_branch_is_excluded() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();

        // Commit on the main branch and tag it v1.0.0.
        let base_oid = add_commit(&repo, "initial commit");
        tag_oid(&repo, "v1.0.0", base_oid);
        let main_branch = current_branch_name(&repo);

        // Create a divergent branch from that same base commit,
        // add a commit there, and tag it v2.0.0.
        {
            let base_commit = repo.find_commit(base_oid).unwrap();
            repo.branch("other", &base_commit, false).unwrap();
        }
        repo.set_head("refs/heads/other").unwrap();
        let divergent_oid = add_commit(&repo, "divergent commit");
        tag_oid(&repo, "v2.0.0", divergent_oid);

        // Restore HEAD to the main branch before handing off to
        // LocalRepo so it detects the correct default branch.
        repo.set_head(&format!("refs/heads/{main_branch}")).unwrap();

        let forge = LocalRepo::new(dir.path(), None).unwrap();

        // Querying main_branch must return v1.0.0 only; v2.0.0 is
        // not in main_branch's history.
        let mut result = forge
            .get_latest_tags_for_prefix("v", &main_branch)
            .await
            .unwrap();
        result.sort_by(|a, b| b.semver.cmp(&a.semver));

        assert!(
            !result.is_empty(),
            "v1.0.0 (ancestor of main) should be found"
        );
        assert_eq!(
            result[0].name, "v1.0.0",
            "v2.0.0 (only on divergent branch) must be excluded"
        );
    }

    #[tokio::test]
    async fn create_release_branch_always_returns_to_starting_branch() {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        configure_git_user(&repo);
        add_commit(&repo, "initial commit");
        add_commit(&repo, "feat: main branch feature");
        let base_branch = current_branch_name(&repo);

        let test_branch = "test";
        create_branch(&repo, test_branch, &base_branch).unwrap();
        switch_branch(&repo, test_branch).unwrap();
        add_commit(&repo, "fix: test commit");

        let remote = Remote {
            forge: Arc::new(MockForge::new()),
            token: SecretString::from("token"),
            url: RepoUrl {
                host: "host".into(),
                name: "test-repo".into(),
                owner: "test".into(),
                path: "test/test-repo".into(),
                port: None,
                scheme: Scheme::Http,
                token: None,
            },
        };

        let mut local_forge = LocalRepo::new(dir.path(), Some(remote)).unwrap();
        local_forge.disable_push_targets();

        local_forge
            .create_release_branch(CreateReleaseBranchRequest {
                base_branch,
                release_branch: "release-main".into(),
                message: "chore(main): release test".into(),
                file_changes: vec![FileChange {
                    content: "content".into(),
                    path: "CHANGELOG.md".into(),
                    update_type: FileUpdateType::Prepend,
                }],
            })
            .await
            .unwrap();

        drop(local_forge);

        let repo = git2::Repository::open(dir.path()).unwrap();
        let current_branch = current_branch_name(&repo);
        assert_eq!(current_branch, test_branch);
    }
}
