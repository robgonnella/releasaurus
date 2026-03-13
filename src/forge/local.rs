//! Local forge implementation for offline development and testing.
use async_trait::async_trait;
use color_eyre::eyre::{Context, OptionExt};
use git2::{BranchType, Commit as Git2Commit, Sort, TreeWalkMode};
use regex::Regex;
use std::{
    path::{self, Path, PathBuf},
    sync::Arc,
};
use tokio::{fs, sync::Mutex};
use url::Url;

use crate::{
    Result,
    analyzer::release::Tag,
    config::{Config, DEFAULT_CONFIG_FILE},
    error::ReleasaurusError,
    forge::{
        request::{
            Commit, CreateCommitRequest, CreatePrRequest,
            CreateReleaseBranchRequest, ForgeCommit, GetFileContentRequest,
            GetPrRequest, PrLabelsRequest, PullRequest, ReleaseByTagResponse,
            UpdatePrRequest,
        },
        traits::Forge,
    },
};

/// LocalRepo forge implementation using git2 for local repository operations.
pub struct LocalRepo {
    repo_path: PathBuf,
    repo_name: String,
    repo: Arc<Mutex<git2::Repository>>,
    default_branch: String,
    link_base_url: Url,
}

impl LocalRepo {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let repo_str = repo_path.to_string_lossy();
        let abs_repo_path = path::absolute(repo_path)?;

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

        let repo = git2::Repository::init(repo_path)?;

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
        })
    }
}

impl LocalRepo {}

#[async_trait]
impl Forge for LocalRepo {
    fn repo_name(&self) -> String {
        self.repo_name.clone()
    }

    fn default_branch(&self) -> String {
        self.default_branch.clone()
    }

    fn release_link_base_url(&self) -> Url {
        self.link_base_url.clone()
    }

    fn compare_link_base_url(&self) -> Url {
        self.link_base_url.clone()
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
        _tag: &str,
    ) -> Result<ReleaseByTagResponse> {
        Err(ReleasaurusError::forge("not implemented for local forge"))
    }

    async fn get_latest_tag_for_prefix(
        &self,
        prefix: &str,
        branch: &str,
    ) -> Result<Option<Tag>> {
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
            return Ok(None);
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

        if commits.is_empty() {
            return Ok(None);
        }

        // sort commits by time descending so the first one should contain
        // the latest tag
        commits.sort_by(|(c1, _), (c2, _)| c2.time().cmp(&c1.time()));

        let (_, tag) = commits[0].clone();

        Ok(Some(tag))
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
        log::warn!("local_mode: would create branch: req: {:#?}", req);
        Ok(Commit { sha: "None".into() })
    }

    async fn create_commit(&self, req: CreateCommitRequest) -> Result<Commit> {
        log::warn!("local_mode: would create commit: req: {:#?}", req);
        Ok(Commit { sha: "None".into() })
    }

    async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()> {
        log::warn!(
            "local_mode: would tag commit: tag_name: {tag_name}, sha: {sha}"
        );
        Ok(())
    }

    async fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        log::warn!(
            "local_mode: would request open release pr: req: {:#?}",
            req
        );
        Ok(None)
    }

    async fn get_merged_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        log::warn!(
            "local_mode: would request merged release pr: req: {:#?}",
            req
        );
        Ok(None)
    }

    async fn create_pr(&self, req: CreatePrRequest) -> Result<PullRequest> {
        log::warn!("local_mode: would create release pr: req: {:#?}", req);
        Ok(PullRequest {
            number: 0,
            sha: "None".into(),
            body: req.body,
        })
    }

    async fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        log::warn!("local_mode: would update release pr: req: {:#?}", req);
        Ok(())
    }

    async fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        log::warn!("local_mode: would replace pr labels: req: {:#?}", req);
        Ok(())
    }

    async fn create_release(
        &self,
        tag: &str,
        sha: &str,
        notes: &str,
    ) -> Result<()> {
        log::warn!(
            "local_mode: would create release: tag: {tag}, sha: {sha}, notes: {notes}"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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

    fn tag_oid(repo: &git2::Repository, name: &str, oid: git2::Oid) {
        let obj = repo.find_object(oid, None).unwrap();
        repo.tag_lightweight(name, &obj, false).unwrap();
    }

    fn default_branch(repo: &git2::Repository) -> String {
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
        let branch = default_branch(&repo);
        drop(repo);

        let forge = LocalRepo::new(dir.path()).unwrap();
        let result =
            forge.get_latest_tag_for_prefix("v", &branch).await.unwrap();

        assert!(result.is_some(), "tag at branch head should be found");
        assert_eq!(result.unwrap().name, "v1.0.0");
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
        let main_branch = default_branch(&repo);

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
        drop(repo);

        let forge = LocalRepo::new(dir.path()).unwrap();

        // Querying main_branch must return v1.0.0 only; v2.0.0 is
        // not in main_branch's history.
        let result = forge
            .get_latest_tag_for_prefix("v", &main_branch)
            .await
            .unwrap();

        assert!(
            result.is_some(),
            "v1.0.0 (ancestor of main) should be found"
        );
        assert_eq!(
            result.unwrap().name,
            "v1.0.0",
            "v2.0.0 (only on divergent branch) must be excluded"
        );
    }
}
