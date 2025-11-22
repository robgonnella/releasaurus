//! Implements the Forge trait for Github
use async_trait::async_trait;
use chrono::DateTime;
use color_eyre::eyre::{OptionExt, eyre};
use log::*;
use octocrab::{
    Octocrab,
    models::repos::Object,
    params::{self, repos::Reference},
};
use regex::Regex;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::{cmp, sync::Arc};
use tokio::sync::Mutex;

const COMMITS_QUERY: &str = r#"
query GetCommits($owner: String!, $repo: String!, $page_limit: Int!, $since: GitTimestamp, $cursor: String) {
  repository(owner: $owner, name: $repo) {
    defaultBranchRef {
      target {
        ... on Commit {
          history(first: $page_limit, after: $cursor, since: $since) {
            pageInfo {
              hasNextPage
              endCursor
            }
            edges {
              node {
                oid
                message
                committedDate
                author {
                  name
                  email
                }
                parents {
                  totalCount
                }
                committedDate
                tree {
                  entries {
                    path
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}"#;

const SHA_DATE_QUERY: &str = r#"
query GetShaDate($owner: String!, $repo: String!, $sha: GitObjectID!) {
  repository(owner: $owner, name: $repo) {
    startCommit: object(oid: $sha) {
      ... on Commit {
        committedDate
      }
    }
  }
}"#;

use crate::{
    analyzer::release::Tag,
    config::{Config, DEFAULT_CONFIG_FILE},
    forge::{
        config::{
            DEFAULT_COMMIT_SEARCH_DEPTH, DEFAULT_LABEL_COLOR,
            DEFAULT_PAGE_SIZE, PENDING_LABEL, RemoteConfig,
        },
        request::{
            Commit, CreateBranchRequest, CreatePrRequest, FileUpdateType,
            ForgeCommit, GetPrRequest, PrLabelsRequest, PullRequest,
            UpdatePrRequest,
        },
        traits::Forge,
    },
    result::Result,
};

#[derive(Debug, Deserialize)]
struct StartCommit {
    #[serde(rename = "committedDate")]
    pub committed_date: String,
}

#[derive(Debug, Deserialize)]
struct StartCommitRepo {
    #[serde(rename = "startCommit")]
    pub start_commit: StartCommit,
}

#[derive(Debug, Deserialize)]
struct StartCommitData {
    pub repository: StartCommitRepo,
}

#[derive(Debug, Deserialize)]
struct StartCommitResult {
    pub data: StartCommitData,
}

#[derive(Debug, Deserialize)]
struct CommitsQueryParents {
    #[serde(rename = "totalCount")]
    total_count: u64,
}

#[derive(Debug, Deserialize)]
struct CommitsQueryAuthor {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
struct CommitsQueryTreeEntry {
    pub path: String,
}

#[derive(Debug, Deserialize)]
struct CommitsQueryTree {
    pub entries: Vec<CommitsQueryTreeEntry>,
}

#[derive(Debug, Deserialize)]
struct CommitsQueryNode {
    pub oid: String,
    pub message: String,
    #[serde(rename = "committedDate")]
    pub committed_date: String,
    pub author: CommitsQueryAuthor,
    pub parents: CommitsQueryParents,
    pub tree: CommitsQueryTree,
}

#[derive(Debug, Deserialize)]
struct CommitsQueryEdge {
    pub node: CommitsQueryNode,
}

#[derive(Debug, Deserialize)]
struct QueryPageInfo {
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
    #[serde(rename = "endCursor")]
    pub end_cursor: String,
}

#[derive(Debug, Deserialize)]
struct CommitsQueryHistory {
    pub edges: Vec<CommitsQueryEdge>,
    #[serde(rename = "pageInfo")]
    pub page_info: QueryPageInfo,
}

#[derive(Debug, Deserialize)]
struct CommitsQueryTarget {
    pub history: CommitsQueryHistory,
}

#[derive(Debug, Deserialize)]
struct CommitsQueryDefaultBranch {
    pub target: CommitsQueryTarget,
}

#[derive(Debug, Deserialize)]
struct CommitsQueryRepository {
    #[serde(rename = "defaultBranchRef")]
    pub default_branch_ref: CommitsQueryDefaultBranch,
}

#[derive(Debug, Deserialize)]
struct CommitsQueryData {
    pub repository: CommitsQueryRepository,
}

#[derive(Debug, Deserialize)]
struct CommitsQueryResult {
    pub data: CommitsQueryData,
}

#[derive(Debug, Serialize)]
struct CommitsQueryVariables {
    pub owner: String,
    pub repo: String,
    pub cursor: Option<String>,
    pub since: Option<String>,
    pub page_limit: u64,
}

#[derive(Debug, Serialize)]
struct ShaDateQueryVariables {
    pub owner: String,
    pub repo: String,
    pub sha: String,
}

#[derive(Debug, Serialize)]
struct GithubTreeEntry {
    pub path: String,
    pub mode: String,
    pub content: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Serialize)]
struct GithubTree {
    pub base_tree: String,
    pub tree: Vec<GithubTreeEntry>,
}

#[derive(Debug, Deserialize)]
struct Tree {
    pub sha: String,
}

pub const TREE_BLOB_MODE: &str = "100644";
pub const TREE_BLOB_TYPE: &str = "blob";

/// GitHub forge implementation using Octocrab for API interactions with
/// commit history, tags, PRs, and releases.
pub struct Github {
    config: RemoteConfig,
    commit_search_depth: Arc<Mutex<u64>>,
    base_uri: String,
    instance: Octocrab,
    default_branch: String,
}

impl Github {
    /// Create GitHub client with personal access token authentication and API
    /// base URL configuration.
    pub async fn new(config: RemoteConfig) -> Result<Self> {
        let base_uri = format!("{}://api.{}", config.scheme, config.host);
        let builder = Octocrab::builder()
            .personal_token(config.token.clone())
            .base_uri(base_uri.clone())?;
        let instance = builder.build()?;

        let repo = instance.repos(&config.owner, &config.repo).get().await?;
        let err_msg = format!(
            "failed to find default branch for gitea repo: {}",
            config.path
        );
        let default_branch = repo.default_branch.ok_or_eyre(err_msg)?;

        Ok(Self {
            config,
            commit_search_depth: Arc::new(Mutex::new(
                DEFAULT_COMMIT_SEARCH_DEPTH,
            )),
            base_uri,
            instance,
            default_branch,
        })
    }

    async fn create_tree(&self, tree: GithubTree) -> Result<Tree> {
        let endpoint = format!(
            "{}/repos/{}/{}/git/trees",
            self.base_uri, self.config.owner, self.config.repo
        );

        let body = serde_json::json!(tree);

        info!("creating tree starting from: {}", tree.base_tree);

        let tree: Tree = self.instance.post(endpoint, Some(&body)).await?;

        info!("created new tree: {}", tree.sha);

        Ok(tree)
    }

    async fn get_tree_entries(
        &self,
        req: CreateBranchRequest,
    ) -> Result<Vec<GithubTreeEntry>> {
        let mut entries: Vec<GithubTreeEntry> = vec![];

        for change in req.file_changes.into_iter() {
            let mut content = change.content;
            if matches!(change.update_type, FileUpdateType::Prepend)
                && let Some(existing_content) =
                    self.get_file_content(&change.path).await?
            {
                content = format!("{content}{existing_content}");
            }
            let path = change
                .path
                .strip_prefix("./")
                .unwrap_or(&change.path)
                .to_string();

            entries.push(GithubTreeEntry {
                path,
                mode: TREE_BLOB_MODE.into(),
                kind: TREE_BLOB_TYPE.into(),
                content,
            });
        }

        Ok(entries)
    }

    async fn create_commit(
        &self,
        message: &str,
        parent_sha: &str,
        tree_sha: &str,
    ) -> Result<Commit> {
        let endpoint = format!(
            "{}/repos/{}/{}/git/commits",
            self.base_uri, self.config.owner, self.config.repo
        );

        let parents = serde_json::json!(vec![parent_sha.to_string()]);

        let body = serde_json::json!({
          "message": message.to_string(),
          "tree": tree_sha.to_string(),
          "parents": parents,
        });

        let commit: Commit = self.instance.post(endpoint, Some(&body)).await?;

        Ok(commit)
    }
}

#[async_trait]
impl Forge for Github {
    fn repo_name(&self) -> String {
        self.config.repo.clone()
    }

    fn remote_config(&self) -> RemoteConfig {
        self.config.clone()
    }

    fn default_branch(&self) -> String {
        self.default_branch.clone()
    }

    async fn load_config(&self) -> Result<Config> {
        if let Some(content) =
            self.get_file_content(DEFAULT_CONFIG_FILE).await?
        {
            let config: Config = toml::from_str(&content)?;

            let mut config_search_depth = config.first_release_search_depth;

            if config_search_depth == 0 {
                config_search_depth = u64::MAX;
            }

            let mut search_depth = self.commit_search_depth.lock().await;
            *search_depth = config_search_depth;

            Ok(config)
        } else {
            info!("no configuration found: using default");
            Ok(Config::default())
        }
    }

    async fn get_file_content(&self, path: &str) -> Result<Option<String>> {
        let result = self
            .instance
            .repos(&self.config.owner, &self.config.repo)
            .get_content()
            .path(path)
            .send()
            .await;

        match result {
            Err(octocrab::Error::GitHub { source, backtrace }) => {
                if source.status_code == StatusCode::NOT_FOUND {
                    info!("no file found for path: {path}");
                    Ok(None)
                } else {
                    let msg = format!(
                        "error getting contents for path: {path}, status: {}, backtrace: {backtrace}",
                        source.status_code
                    );
                    error!("{msg}");
                    Err(eyre!(msg))
                }
            }
            Err(err) => {
                let msg = format!(
                    "encountered error getting file contents for path: {path}: {err}"
                );
                error!("{msg}");
                Err(eyre!(msg))
            }
            Ok(mut data) => {
                let items = data.take_items();

                if items.is_empty() {
                    info!("no file found for path: {path}");
                    return Ok(None);
                }

                if let Some(content) = items[0].decoded_content() {
                    Ok(Some(content))
                } else {
                    Err(eyre!("failed to decode file content for path: {path}"))
                }
            }
        }
    }

    async fn get_latest_tag_for_prefix(
        &self,
        prefix: &str,
    ) -> Result<Option<Tag>> {
        let re = Regex::new(format!(r"^{prefix}").as_str())?;

        let page = self
            .instance
            .repos(&self.config.owner, &self.config.repo)
            .list_tags()
            .send()
            .await?;

        for tag in page.into_iter() {
            if re.is_match(&tag.name) {
                let stripped = re.replace_all(&tag.name, "").to_string();
                if let Ok(sver) = semver::Version::parse(&stripped) {
                    // get tag timestamp
                    let vars = ShaDateQueryVariables {
                        owner: self.config.owner.clone(),
                        repo: self.config.repo.clone(),
                        sha: tag.commit.sha.clone(),
                    };

                    let json = serde_json::json!({
                      "query": SHA_DATE_QUERY,
                      "variables": vars,
                    });

                    let result: StartCommitResult =
                        self.instance.graphql(&json).await?;

                    let created =
                        result.data.repository.start_commit.committed_date;

                    return Ok(Some(Tag {
                        name: tag.name,
                        semver: sver,
                        sha: tag.commit.sha,
                        timestamp: DateTime::parse_from_rfc3339(&created)
                            .unwrap()
                            .timestamp(),
                    }));
                }
            }
        }

        Ok(None)
    }

    async fn get_commits(
        &self,
        sha: Option<String>,
    ) -> Result<Vec<ForgeCommit>> {
        let search_depth = self.commit_search_depth.lock().await;
        let page_limit = cmp::min(DEFAULT_PAGE_SIZE.into(), *search_depth);
        let mut commits: Vec<ForgeCommit> = vec![];
        let mut since_date = None;

        if let Some(sha) = sha.clone() {
            // get commits since sha
            let vars = ShaDateQueryVariables {
                owner: self.config.owner.clone(),
                repo: self.config.repo.clone(),
                sha,
            };

            let json = serde_json::json!({
              "query": SHA_DATE_QUERY,
              "variables": vars,
            });

            let result: StartCommitResult =
                self.instance.graphql(&json).await?;

            since_date =
                Some(result.data.repository.start_commit.committed_date)
        }

        let mut cursor = None;
        let mut has_more = true;
        let search_depth = *search_depth as usize;

        while has_more {
            if sha.is_none() && commits.len() >= search_depth {
                break;
            }

            let vars = CommitsQueryVariables {
                owner: self.config.owner.clone(),
                repo: self.config.repo.clone(),
                cursor: cursor.clone(),
                since: since_date.clone(),
                page_limit,
            };

            let result: CommitsQueryResult = self.instance
                    .graphql(&serde_json::json!({ "query": COMMITS_QUERY, "variables": vars }))
                    .await?;

            let edges = result
                .data
                .repository
                .default_branch_ref
                .target
                .history
                .edges;

            let forge_commits = edges
                .iter()
                // filter out starting sha: we only want commits after starting sha
                // filter out merge commits
                .filter(|e| {
                    if let Some(sha) = sha.clone() {
                        e.node.oid != sha
                    } else {
                        true
                    }
                })
                .map(|e| ForgeCommit {
                    author_email: e.node.author.email.clone(),
                    author_name: e.node.author.name.clone(),
                    id: e.node.oid.clone(),
                    short_id: e
                        .node
                        .oid
                        .clone()
                        .split("")
                        .take(8)
                        .collect::<Vec<&str>>()
                        .join(""),
                    link: format!(
                        "{}/{}",
                        self.config.commit_link_base_url, e.node.oid,
                    ),
                    merge_commit: e.node.parents.total_count > 1,
                    message: e.node.message.clone(),
                    timestamp: DateTime::parse_from_rfc3339(
                        &e.node.committed_date,
                    )
                    .unwrap()
                    .timestamp(),
                    files: e
                        .node
                        .tree
                        .entries
                        .iter()
                        .map(|e| e.path.clone())
                        .collect::<Vec<String>>(),
                })
                .collect::<Vec<ForgeCommit>>();

            commits.extend(forge_commits);

            if !result
                .data
                .repository
                .default_branch_ref
                .target
                .history
                .page_info
                .end_cursor
                .is_empty()
            {
                cursor = Some(
                    result
                        .data
                        .repository
                        .default_branch_ref
                        .target
                        .history
                        .page_info
                        .end_cursor,
                );
            }

            has_more = result
                .data
                .repository
                .default_branch_ref
                .target
                .history
                .page_info
                .has_next_page
        }

        Ok(commits)
    }

    async fn create_release_branch(
        &self,
        req: CreateBranchRequest,
    ) -> Result<Commit> {
        let default_branch = self.default_branch();

        let default_ref = self
            .instance
            .repos(&self.config.owner, &self.config.repo)
            .get_ref(&params::repos::Reference::Branch(default_branch))
            .await?;

        let default_sha = match default_ref.object {
            Object::Commit { sha, .. } => Ok(sha),
            _ => Err(eyre!("failed to find sha of default branch")),
        }?;

        let tree = self.get_tree_entries(req.clone()).await?;

        let tree = self
            .create_tree(GithubTree {
                base_tree: default_sha.clone(),
                tree,
            })
            .await?;

        let commit = self
            .create_commit(&req.message, &default_sha, &tree.sha)
            .await?;

        info!("created commit for branch: sha: {}", commit.sha);

        let target_ref = self
            .instance
            .repos(&self.config.owner, &self.config.repo)
            .get_ref(&params::repos::Reference::Branch(req.branch.clone()))
            .await;

        if target_ref.is_ok() {
            info!("release branch {} already exists: updating", req.branch);
            let endpoint = format!(
                "{}/repos/{}/{}/git/refs/heads/{}",
                self.base_uri, self.config.owner, self.config.repo, req.branch
            );
            let _: serde_json::Value = self
                .instance
                .patch(
                    endpoint,
                    Some(&serde_json::json!({
                      "sha": commit.sha,
                      "force": true
                    })),
                )
                .await?;

            return Ok(commit);
        }

        info!("creating release branch {}", req.branch);

        self.instance
            .repos(&self.config.owner, &self.config.repo)
            .create_ref(
                &params::repos::Reference::Branch(req.branch),
                commit.sha.clone(),
            )
            .await?;

        Ok(commit)
    }

    async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()> {
        self.instance
            .repos(&self.config.owner, &self.config.repo)
            .create_ref(&Reference::Tag(tag_name.to_string()), sha)
            .await?;

        Ok(())
    }

    async fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        let prs = self
            .instance
            .pulls(&self.config.owner, &self.config.repo)
            .list()
            .state(params::State::Open)
            .head(req.head_branch)
            .send()
            .await?;

        for pr in prs {
            if let Some(labels) = pr.labels
                && let Some(_pending_label) =
                    labels.iter().find(|l| l.name == PENDING_LABEL)
            {
                return Ok(Some(PullRequest {
                    number: pr.number,
                    sha: pr.head.sha,
                    body: pr.body.unwrap_or_default(),
                }));
            }
        }

        Ok(None)
    }

    async fn get_merged_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        let issues_handler =
            self.instance.issues(&self.config.owner, &self.config.repo);

        info!("looking for closed release prs with pending label");

        let issues = issues_handler
            .list()
            .direction(params::Direction::Descending)
            .labels(&[PENDING_LABEL.into()])
            .state(params::State::Closed)
            .send()
            .await?;

        if issues.items.is_empty() {
            warn!(
                r"No merged release PRs with the label {PENDING_LABEL} found for branch {}. Nothing to release",
                req.head_branch
            );
            return Ok(None);
        }

        for issue in issues {
            let pr = self
                .instance
                .pulls(&self.config.owner, &self.config.repo)
                .get(issue.number)
                .await?;

            if let Some(label) = pr.head.label
                && label == req.head_branch
            {
                if let Some(merged) = pr.merged
                    && !merged
                {
                    return Err(eyre!(format!(
                        "found release PR {} but it hasn't been merged yet",
                        pr.number
                    )));
                }

                info!(
                    "found release pr: {} for branch {}",
                    issue.number, req.head_branch
                );

                let sha = pr
                    .merge_commit_sha
                    .ok_or(eyre!("no merge_commit_sha found for pr"))?;

                return Ok(Some(PullRequest {
                    number: pr.number,
                    sha,
                    body: pr.body.unwrap_or_default(),
                }));
            }
        }

        Ok(None)
    }

    async fn create_pr(&self, req: CreatePrRequest) -> Result<PullRequest> {
        let pr = self
            .instance
            .pulls(&self.config.owner, &self.config.repo)
            .create(req.title, req.head_branch, req.base_branch)
            .body(req.body)
            .send()
            .await?;

        Ok(PullRequest {
            number: pr.number,
            sha: pr.head.sha,
            body: pr.body.unwrap_or_default(),
        })
    }

    async fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        self.instance
            .pulls(&self.config.owner, &self.config.repo)
            .update(req.pr_number)
            .title(req.title)
            .body(req.body)
            .send()
            .await?;

        Ok(())
    }

    async fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        let all_labels = self
            .instance
            .issues(&self.config.owner, &self.config.repo)
            .list_labels_for_repo()
            .per_page(100)
            .send()
            .await?;

        let mut labels = vec![];

        for name in req.labels {
            if let Some(label) =
                all_labels.items.iter().find(|l| l.name == name)
            {
                labels.push(label.name.clone())
            } else {
                let label = self
                    .instance
                    .issues(&self.config.owner, &self.config.repo)
                    .create_label(name, DEFAULT_LABEL_COLOR, "")
                    .await?;
                labels.push(label.name);
            }
        }

        self.instance
            .issues(&self.config.owner, &self.config.repo)
            .replace_all_labels(req.pr_number, &labels)
            .await?;

        Ok(())
    }

    async fn create_release(
        &self,
        tag: &str,
        sha: &str,
        notes: &str,
    ) -> Result<()> {
        self.instance
            .repos(&self.config.owner, &self.config.repo)
            .releases()
            .create(tag)
            .name(tag)
            .body(notes)
            .target_commitish(sha)
            .draft(false)
            .prerelease(false)
            .send()
            .await?;

        Ok(())
    }
}
