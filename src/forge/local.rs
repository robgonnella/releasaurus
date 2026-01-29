//! Local forge implementation for offline development and testing.
use async_trait::async_trait;
use color_eyre::eyre::{Context, OptionExt};
use git2::{Commit as Git2Commit, Sort, TreeWalkMode};
use regex::Regex;
use std::{
    env,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{fs, sync::Mutex};

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

/// LocalRepo forge implementation using .
pub struct LocalRepo {
    repo_path: String,
    repo_name: String,
    repo: Arc<Mutex<git2::Repository>>,
    default_branch: String,
}

impl LocalRepo {
    pub fn new(repo_path: String) -> Result<Self> {
        let mut repo_path_buf = Path::new(&repo_path).to_path_buf();

        if repo_path == "." || repo_path == "./" {
            repo_path_buf = env::current_dir()?;
        }

        let repo_name = repo_path_buf
            .file_name()
            .ok_or_eyre(
                "unable to determine repository directory name from path",
            )?
            .display()
            .to_string();

        let repo = git2::Repository::init(repo_path.clone())?;

        let head = repo.head()?;

        let default_branch = head
            .shorthand()
            .ok_or_eyre("unable to get current branch for local repo")?
            .to_string();

        drop(head);

        Ok(Self {
            repo_name,
            repo_path,
            repo: Arc::new(Mutex::new(repo)),
            default_branch,
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

    fn release_link_base_url(&self) -> String {
        "".into()
    }

    fn compare_link_base_url(&self) -> String {
        "".into()
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
