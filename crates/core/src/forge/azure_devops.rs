//! Implements the Forge trait for Azure DevOps Git repositories.
//!
//! Azure DevOps Git has no native "release object" concept (its
//! "Releases" feature is part of Azure Pipelines CD, not git). The
//! `create_release` method below is a deliberate no-op that logs at
//! info level; tags and the changelog commit are pushed normally.
use async_trait::async_trait;
use base64::{
    Engine,
    prelude::{BASE64_STANDARD, BASE64_URL_SAFE_NO_PAD},
};
use chrono::DateTime;
use color_eyre::eyre::ContextCompat;
use log::{info, warn};
use regex::Regex;
use reqwest::{
    Client, StatusCode,
    header::{HeaderMap, HeaderValue},
};
use secrecy::{ExposeSecret, SecretString};
use std::cmp;
use std::sync::{LazyLock, Once};
use url::Url;

use crate::{
    config::{
        Config, DEFAULT_CONFIG_FILE,
        repository::{DEFAULT_COMMIT_SEARCH_DEPTH, DEFAULT_TAG_SEARCH_DEPTH},
    },
    forge::{
        azure_devops::types::{
            AzureCommit, AzureCommitChanges, AzureList, AzurePullRequest,
            AzureRef, AzureRepo, Change, ChangeItem, CreateLabel,
            CreatePullRequest, NewContent, Push, PushCommit, PushResponse,
            RefUpdate, UpdatePullRequest,
        },
        config::{
            DEFAULT_PAGE_SIZE, LEGACY_PENDING_LABEL, PENDING_LABEL, RepoUrl,
            TokenVar, USER_AGENT, resolve_token,
        },
        request::{
            Commit, CreateCommitRequest, CreatePrRequest,
            CreateReleaseBranchRequest, FileUpdateType, ForgeCommit,
            GetFileContentRequest, GetPrRequest, PrLabelsRequest,
            PrMetadataBlock, PullRequest, ReleaseByTagResponse, Tag,
            UpdatePrRequest,
        },
        traits::Forge,
    },
    result::{ReleasaurusError, Result},
};

mod types;
pub mod url_parse;

const API_VERSION: &str = "7.1";
const LABELS_API_VERSION: &str = "7.1-preview.1";
const ZERO_SHA: &str = "0000000000000000000000000000000000000000";

static EXPERIMENTAL_WARNING: Once = Once::new();

/// Strips the `Merged PR NNN: ` prefix that Azure DevOps prepends to
/// squash-merge commit messages, recovering the original commit subject.
static AZURE_MERGED_PR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(Merged PR \d+:)\s+").unwrap());

/// Azure DevOps forge implementation using reqwest against the
/// Azure DevOps Git REST API.
///
/// `repo_url.owner` is expected to hold `"{org}/{project}"`.
pub struct AzureDevops {
    url: RepoUrl,
    commit_search_depth: usize,
    tag_search_depth: usize,
    /// API base: `https://dev.azure.com/{org}/{project}/_apis/git/repositories/{repo}/`
    base_url: Url,
    client: Client,
    default_branch: String,
    release_link_base_url: Url,
    compare_link_base_url: Url,
}

impl AzureDevops {
    pub async fn new(
        url: RepoUrl,
        token: Option<SecretString>,
    ) -> Result<Self> {
        EXPERIMENTAL_WARNING.call_once(|| {
            warn!(
                "azure devops forge support is EXPERIMENTAL; \
                 expect rough edges. the release step only pushes the \
                 git tag — azure devops has no native release object \
                 to publish notes against."
            );
        });

        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .ok();

        let token = resolve_token(
            token,
            url.token.as_ref(),
            vec![TokenVar::ReleasaurusAzureDevops, TokenVar::AzureDevops],
        )?;

        let link_base_url = url.link_base_url();

        // Web URL prefix: https://dev.azure.com/{org}/{project}/_git/{repo}
        let web_repo_url = format!("{}{}", link_base_url, url.path);

        // Azure DevOps has no Releases page; link tags to the tag listing.
        let release_link_base_url =
            Url::parse(&format!("{}?path=/&version=GT", web_repo_url))?;
        let compare_link_base_url = Url::parse(&format!(
            "{}/branchCompare?baseVersion=GT",
            web_repo_url
        ))?;

        let mut headers = HeaderMap::new();

        // Azure DevOps accepts either a PAT (Basic base64(":{PAT}")) or an
        // OAuth bearer (typically a pipeline System.AccessToken, which is a
        // signed JWT). Detect the JWT shape to pick the scheme.
        let token_value = if looks_like_jwt(token.expose_secret()) {
            HeaderValue::from_str(&format!("Bearer {}", token.expose_secret()))?
        } else {
            let basic = BASE64_STANDARD
                .encode(format!(":{}", token.expose_secret()).as_bytes());
            HeaderValue::from_str(&format!("Basic {}", basic))?
        };
        headers.append("Authorization", token_value);
        headers.append("Accept", HeaderValue::from_static("application/json"));
        headers
            .append("User-Agent", HeaderValue::from_str(USER_AGENT.as_str())?);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        // API base: https://dev.azure.com/{org}/{project}/_apis/git/repositories/{repo}/
        let base_url = format!(
            "{}/{}/_apis/git/repositories/{}/",
            link_base_url, url.owner, url.name
        );
        let base_url = Url::parse(&base_url)?;

        // Fetch repo metadata for the default branch.
        let mut repo_url = base_url.clone();
        repo_url
            .query_pairs_mut()
            .append_pair("api-version", API_VERSION);
        let response = client.get(repo_url).send().await?;
        let repo: AzureRepo = read_json(response).await?;
        let default_branch = repo
            .default_branch
            .as_deref()
            .map(strip_refs_heads)
            .map(str::to_string)
            .wrap_err("failed to get default branch")?;

        Ok(Self {
            url,
            commit_search_depth: DEFAULT_COMMIT_SEARCH_DEPTH,
            tag_search_depth: DEFAULT_TAG_SEARCH_DEPTH,
            client,
            base_url,
            release_link_base_url,
            compare_link_base_url,
            default_branch,
        })
    }

    fn endpoint(&self, path: &str) -> Result<Url> {
        let mut url = self.base_url.join(path)?;
        url.query_pairs_mut()
            .append_pair("api-version", API_VERSION);
        Ok(url)
    }

    /// Returns true if `commit_sha` is reachable from `branch`'s head
    /// (i.e. the commit is an ancestor of the branch). Uses the Azure
    /// DevOps diffs API: if `commonCommit == baseCommit`, the base
    /// (commit) is fully contained in the target (branch).
    async fn is_ancestor_of_branch(
        &self,
        commit_sha: &str,
        branch: &str,
    ) -> Result<bool> {
        let mut url = self.base_url.join("diffs/commits")?;
        url.query_pairs_mut()
            .append_pair("api-version", API_VERSION)
            .append_pair("baseVersion", commit_sha)
            .append_pair("baseVersionType", "commit")
            .append_pair("targetVersion", branch)
            .append_pair("targetVersionType", "branch");
        let response = self.client.get(url).send().await?;
        // 404 means the base commit isn't reachable from the target branch
        // (or doesn't exist) — a legitimate "not an ancestor". Any other
        // non-2xx (401, 429, 5xx) would silently drop reachable tags if
        // coerced to false, so propagate it.
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(false);
        }
        let body: serde_json::Value = read_json(response).await?;
        let base = body.get("baseCommit").and_then(|v| v.as_str());
        let common = body.get("commonCommit").and_then(|v| v.as_str());
        match (base, common) {
            (Some(b), Some(c)) => Ok(b == c),
            _ => Ok(false),
        }
    }

    async fn get_branch_head_sha(&self, branch: &str) -> Result<String> {
        let mut refs_url = self.base_url.join("refs")?;
        refs_url
            .query_pairs_mut()
            .append_pair("api-version", API_VERSION)
            .append_pair("filter", &format!("heads/{branch}"));
        let response = self.client.get(refs_url).send().await?;
        let refs: AzureList<AzureRef> = read_json(response).await?;
        let want = format!("refs/heads/{branch}");
        refs.value
            .into_iter()
            .find(|r| r.name == want)
            .map(|r| r.object_id)
            .ok_or_else(|| {
                ReleasaurusError::forge(format!("branch not found: {branch}"))
            })
    }

    async fn build_push_changes(
        &self,
        branch: &str,
        changes: &[crate::forge::request::FileChange],
    ) -> Result<Vec<Change>> {
        let mut out = vec![];
        for change in changes.iter() {
            let existing = self
                .get_file_content(GetFileContentRequest {
                    branch: Some(branch.to_string()),
                    path: change.path.clone(),
                })
                .await?;

            let mut content = change.content.clone();
            let (change_type, existed) = match existing.as_deref() {
                Some(prev) => {
                    if matches!(change.update_type, FileUpdateType::Prepend) {
                        content = format!("{content}\n{prev}");
                    }
                    ("edit", Some(prev.to_string()))
                }
                None => ("add", None),
            };

            if let Some(prev) = existed.as_ref()
                && content == *prev
            {
                warn!(
                    "skipping file update content matches existing state: {}",
                    change.path
                );
                continue;
            }

            out.push(Change {
                change_type: change_type.to_string(),
                item: ChangeItem {
                    path: normalize_path(&change.path),
                },
                new_content: Some(NewContent {
                    content,
                    content_type: "rawtext".into(),
                }),
            });
        }
        Ok(out)
    }

    async fn push_to_branch(
        &self,
        branch: &str,
        old_sha: &str,
        message: &str,
        changes: Vec<Change>,
    ) -> Result<String> {
        let url = self.endpoint("pushes")?;
        let body = Push {
            ref_updates: vec![RefUpdate {
                name: format!("refs/heads/{branch}"),
                old_object_id: old_sha.to_string(),
                new_object_id: None,
            }],
            commits: vec![PushCommit {
                comment: message.to_string(),
                changes,
            }],
        };
        let response = self.client.post(url).json(&body).send().await?;
        let push: PushResponse = read_json(response).await?;
        push.commits
            .into_iter()
            .next()
            .map(|c| c.commit_id)
            .ok_or_else(|| {
                ReleasaurusError::forge("push returned no commits".to_string())
            })
    }

    /// Fetches a single PR by ID, returning its full description (the list
    /// endpoint truncates `description` at ~400 chars).
    async fn get_pr_by_id(&self, pr_id: u64) -> Result<AzurePullRequest> {
        let url = self.endpoint(&format!("pullrequests/{pr_id}"))?;
        let response = self.client.get(url).send().await?;
        let pr: AzurePullRequest = read_json(response).await?;
        Ok(pr)
    }

    async fn find_pr_by_branch(
        &self,
        status: &str,
        head_branch: &str,
    ) -> Result<Vec<AzurePullRequest>> {
        let mut found = vec![];
        let mut skip: u64 = 0;
        let page_size = u64::from(DEFAULT_PAGE_SIZE);
        loop {
            let mut url = self.base_url.join("pullrequests")?;
            url.query_pairs_mut()
                .append_pair("api-version", API_VERSION)
                .append_pair("searchCriteria.status", status)
                .append_pair(
                    "searchCriteria.sourceRefName",
                    &format!("refs/heads/{head_branch}"),
                )
                .append_pair("$top", &page_size.to_string())
                .append_pair("$skip", &skip.to_string());
            let response = self.client.get(url).send().await?;
            let list: AzureList<AzurePullRequest> = read_json(response).await?;
            let count = list.value.len() as u64;
            found.extend(list.value);
            if count < page_size {
                break;
            }
            skip += page_size;
        }
        Ok(found)
    }

    async fn pr_has_label(&self, pr_number: u64, label: &str) -> Result<bool> {
        let labels = self.get_pr_labels(pr_number).await?;
        Ok(labels.iter().any(|l| l.name == label && l.active))
    }

    async fn get_pr_labels(
        &self,
        pr_number: u64,
    ) -> Result<Vec<crate::forge::azure_devops::types::AzureLabel>> {
        let mut url = self
            .base_url
            .join(&format!("pullrequests/{pr_number}/labels"))?;
        url.query_pairs_mut()
            .append_pair("api-version", LABELS_API_VERSION);
        let response = self.client.get(url).send().await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(vec![]);
        }
        let list: AzureList<crate::forge::azure_devops::types::AzureLabel> =
            read_json(response).await?;
        Ok(list.value)
    }

    async fn delete_pr_label(
        &self,
        pr_number: u64,
        label_id_or_name: &str,
    ) -> Result<()> {
        let mut url = self.base_url.join(&format!(
            "pullrequests/{pr_number}/labels/{label_id_or_name}"
        ))?;
        url.query_pairs_mut()
            .append_pair("api-version", LABELS_API_VERSION);
        let response = self.client.delete(url).send().await?;
        response.error_for_status()?;
        Ok(())
    }

    async fn add_pr_label(&self, pr_number: u64, name: &str) -> Result<()> {
        let mut url = self
            .base_url
            .join(&format!("pullrequests/{pr_number}/labels"))?;
        url.query_pairs_mut()
            .append_pair("api-version", LABELS_API_VERSION);
        let body = CreateLabel {
            name: name.to_string(),
        };
        let response = self.client.post(url).json(&body).send().await?;
        response.error_for_status()?;
        Ok(())
    }

    async fn get_commit_timestamp(&self, commit_id: &str) -> Result<i64> {
        let mut url = self.base_url.join(&format!("commits/{commit_id}"))?;
        url.query_pairs_mut()
            .append_pair("api-version", API_VERSION);
        let response = self.client.get(url).send().await?;
        let commit: AzureCommit = read_json(response).await?;
        Ok(DateTime::parse_from_rfc3339(&commit.author.date)
            .map(|t| t.timestamp())
            .unwrap_or(0))
    }

    async fn get_commit_files(&self, commit_id: &str) -> Result<Vec<String>> {
        let mut url = self
            .base_url
            .join(&format!("commits/{commit_id}/changes"))?;
        url.query_pairs_mut()
            .append_pair("api-version", API_VERSION);
        let response = self.client.get(url).send().await?;
        // 404 is legitimate (commit has no recorded change list); other
        // non-2xx codes (401/429/5xx) would silently produce empty file
        // lists and mask auth or transport problems, so propagate.
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(vec![]);
        }
        let changes: AzureCommitChanges = read_json(response).await?;
        Ok(changes
            .changes
            .into_iter()
            .filter_map(|c| {
                let p = c.item.path;
                if p.is_empty() {
                    None
                } else {
                    Some(p.trim_start_matches('/').to_string())
                }
            })
            .collect())
    }
}

fn strip_refs_heads(name: &str) -> &str {
    name.strip_prefix("refs/heads/").unwrap_or(name)
}

fn strip_refs_tags(name: &str) -> &str {
    name.strip_prefix("refs/tags/").unwrap_or(name)
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.strip_prefix("./").unwrap_or(path);
    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

/// Parses a JSON body from `response`, wrapping parse failures with an
/// actionable hint. Azure DevOps responds to invalid or under-privileged
/// tokens with an HTML sign-in page (HTTP 203 + `text/html`), which slips
/// past `error_for_status` and then fails JSON parsing — the hint points
/// users at the likely auth issue rather than a bare serde error.
async fn read_json<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T> {
    let response = response.error_for_status()?;
    let status = response.status();
    let url = response.url().clone();
    response.json::<T>().await.map_err(|err| {
        ReleasaurusError::forge(format!(
            "failed to parse Azure DevOps response from {url} \
             (HTTP {status}): {err} — the configured token might be \
             invalid or lack access to this repository"
        ))
    })
}

/// Returns `true` if `token` has the structural shape of a JWT (RFC 7519):
/// three base64url segments separated by dots, where the first segment
/// decodes to a JSON object containing an `alg` field. Used to distinguish
/// an Azure DevOps OAuth bearer (e.g. pipeline `System.AccessToken`) from a
/// PAT, which is an opaque base32-style string.
fn looks_like_jwt(token: &str) -> bool {
    // Fast path: every JWT header begins with `{"`, which base64url-encodes
    // to `eyJ`. Skip the parse work for anything that can't be one.
    if !token.starts_with("eyJ") {
        return false;
    }
    let mut parts = token.split('.');
    let (Some(header), Some(_), Some(_), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let Ok(header_bytes) = BASE64_URL_SAFE_NO_PAD.decode(header) else {
        return false;
    };
    let Ok(header_json) =
        serde_json::from_slice::<serde_json::Value>(&header_bytes)
    else {
        return false;
    };
    header_json.get("alg").is_some()
}

#[async_trait]
impl Forge for AzureDevops {
    fn repo_name(&self) -> String {
        self.url.name.clone()
    }

    fn release_link_base_url(&self) -> Url {
        self.release_link_base_url.clone()
    }

    fn compare_link_base_url(&self) -> Url {
        self.compare_link_base_url.clone()
    }

    fn default_branch(&self) -> String {
        self.default_branch.clone()
    }

    fn set_commit_search_depth(&mut self, depth: usize) {
        self.commit_search_depth = if depth == 0 { usize::MAX } else { depth }
    }

    fn set_tag_search_depth(&mut self, depth: usize) {
        self.tag_search_depth = if depth == 0 { usize::MAX } else { depth }
    }

    async fn get_file_content(
        &self,
        req: GetFileContentRequest,
    ) -> Result<Option<String>> {
        let mut url = self.base_url.join("items")?;
        url.query_pairs_mut()
            .append_pair("api-version", API_VERSION)
            .append_pair("path", &normalize_path(&req.path))
            .append_pair("includeContent", "true")
            .append_pair("$format", "text");
        if let Some(branch) = req.branch.as_ref() {
            url.query_pairs_mut()
                .append_pair("versionDescriptor.version", branch)
                .append_pair("versionDescriptor.versionType", "branch");
        }
        let response = self
            .client
            .get(url)
            .header("Accept", "text/plain")
            .send()
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let result = response.error_for_status()?;
        let content = result.text().await?;
        Ok(Some(content))
    }

    async fn load_config(
        &self,
        branch: Option<String>,
        config_path: Option<String>,
    ) -> Result<Config> {
        let is_custom = config_path.is_some();
        let path =
            config_path.unwrap_or_else(|| DEFAULT_CONFIG_FILE.to_string());
        if let Some(content) = self
            .get_file_content(GetFileContentRequest {
                branch,
                path: path.clone(),
            })
            .await?
        {
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else if is_custom {
            Err(ReleasaurusError::invalid_config(format!(
                "configuration file not found at: {path}"
            )))
        } else {
            Ok(Config::default())
        }
    }

    async fn get_release_by_tag(
        &self,
        tag: &str,
    ) -> Result<ReleaseByTagResponse> {
        // Azure DevOps has no native release object. Resolve the tag's
        // commit SHA via the refs endpoint and return empty notes —
        // callers that need notes should source them from the
        // release-PR body.
        let mut url = self.base_url.join("refs")?;
        url.query_pairs_mut()
            .append_pair("api-version", API_VERSION)
            .append_pair("filter", &format!("tags/{tag}"));
        let response = self.client.get(url).send().await?;
        let refs: AzureList<AzureRef> = read_json(response).await?;
        let want = format!("refs/tags/{tag}");
        let r = refs.value.into_iter().find(|r| r.name == want).ok_or_else(
            || ReleasaurusError::forge(format!("tag not found: {tag}")),
        )?;
        Ok(ReleaseByTagResponse {
            tag: tag.to_string(),
            sha: r.object_id,
            notes: String::new(),
        })
    }

    async fn get_latest_tags_for_prefix(
        &self,
        prefix: &str,
        branch: &str,
        starting_sha: Option<String>,
    ) -> Result<Vec<Tag>> {
        let re = Regex::new(format!(r"^{prefix}").as_str())?;
        let mut url = self.base_url.join("refs")?;
        url.query_pairs_mut()
            .append_pair("api-version", API_VERSION)
            .append_pair("filter", "tags/");
        let response = self.client.get(url).send().await?;
        let refs: AzureList<AzureRef> = read_json(response).await?;
        let mut tags = vec![];
        for (count, r) in refs.value.into_iter().enumerate() {
            if count >= self.tag_search_depth {
                break;
            }
            let name = strip_refs_tags(&r.name).to_string();
            if !re.is_match(&name) {
                continue;
            }
            let stripped = re.replace_all(&name, "").to_string();
            let Ok(sver) = semver::Version::parse(&stripped) else {
                continue;
            };
            // Only return tags reachable from the target branch.
            if !self.is_ancestor_of_branch(&r.object_id, branch).await? {
                continue;
            }
            if let Some(sha) = starting_sha.as_ref()
                && r.object_id == *sha
            {
                break;
            }
            let timestamp = self.get_commit_timestamp(&r.object_id).await.ok();
            tags.push(Tag {
                name,
                semver: sver,
                sha: r.object_id,
                timestamp,
            });
        }
        Ok(tags)
    }

    async fn get_commits(
        &self,
        branch: Option<String>,
        sha: Option<String>,
    ) -> Result<Vec<ForgeCommit>> {
        let mut skip: u64 = 0;
        let page_size = cmp::min(
            u64::from(DEFAULT_PAGE_SIZE),
            self.commit_search_depth as u64,
        );
        let mut commits: Vec<ForgeCommit> = vec![];
        let mut count = 0usize;

        loop {
            let mut url = self.base_url.join("commits")?;
            url.query_pairs_mut()
                .append_pair("api-version", API_VERSION)
                .append_pair("$top", &page_size.to_string())
                .append_pair("$skip", &skip.to_string())
                .append_pair("searchCriteria.includeWorkItems", "false");
            // Azure DevOps commits API without a branch filter returns commits
            // from all refs, not just the default branch. Always filter.
            let effective_branch =
                branch.as_deref().unwrap_or(&self.default_branch);
            url.query_pairs_mut()
                .append_pair(
                    "searchCriteria.itemVersion.version",
                    effective_branch,
                )
                .append_pair(
                    "searchCriteria.itemVersion.versionType",
                    "branch",
                );

            let response = self.client.get(url).send().await?;
            let list: AzureList<AzureCommit> = read_json(response).await?;
            let returned = list.value.len() as u64;

            for c in list.value.into_iter() {
                if sha.is_none() && count >= self.commit_search_depth {
                    return Ok(commits);
                }
                if let Some(target) = sha.as_ref()
                    && *target == c.commit_id
                {
                    return Ok(commits);
                }

                // Fetch file list for this commit (best effort). Azure
                // DevOps' commits list endpoint doesn't include change
                // info, so this is one extra request per commit.
                log::debug!(
                    "backfilling file list for commit: {}",
                    c.commit_id
                );
                let files = self
                    .get_commit_files(&c.commit_id)
                    .await
                    .unwrap_or_default();

                let timestamp = DateTime::parse_from_rfc3339(&c.author.date)
                    .map(|t| t.timestamp())
                    .unwrap_or(0);

                commits.push(ForgeCommit {
                    author_email: c.author.email,
                    author_name: c.author.name,
                    id: c.commit_id.clone(),
                    short_id: c.commit_id.chars().take(8).collect(),
                    link: c.remote_url,
                    merge_commit: c.parents.len() > 1,
                    message: AZURE_MERGED_PR_RE
                        .replace(c.comment.trim_end(), "")
                        .into_owned(),
                    timestamp,
                    files,
                });
                count += 1;
            }

            if returned < page_size {
                break;
            }
            skip += page_size;
        }

        Ok(commits)
    }

    async fn create_release_branch(
        &self,
        req: CreateReleaseBranchRequest,
    ) -> Result<Commit> {
        let base_sha = self.get_branch_head_sha(&req.base_branch).await?;
        let existing_sha =
            self.get_branch_head_sha(&req.release_branch).await.ok();
        let old_object_id =
            existing_sha.as_deref().unwrap_or(ZERO_SHA).to_string();
        let url = self.endpoint("refs")?;
        let body = vec![RefUpdate {
            name: format!("refs/heads/{}", req.release_branch),
            old_object_id,
            new_object_id: Some(base_sha.clone()),
        }];
        self.client
            .post(url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        let changes = self
            .build_push_changes(&req.base_branch, &req.file_changes)
            .await?;
        if changes.is_empty() {
            return Ok(Commit { sha: base_sha });
        }
        let new_sha = self
            .push_to_branch(
                &req.release_branch,
                &base_sha,
                &req.message,
                changes,
            )
            .await?;
        Ok(Commit { sha: new_sha })
    }

    async fn create_commit(&self, req: CreateCommitRequest) -> Result<Commit> {
        let head_sha = self.get_branch_head_sha(&req.target_branch).await?;
        let changes = self
            .build_push_changes(&req.target_branch, &req.file_changes)
            .await?;
        if changes.is_empty() {
            warn!(
                "commit would result in no changes: target_branch: {}, message: {}",
                req.target_branch, req.message,
            );
            return Ok(Commit { sha: "None".into() });
        }
        let new_sha = self
            .push_to_branch(
                &req.target_branch,
                &head_sha,
                &req.message,
                changes,
            )
            .await?;
        Ok(Commit { sha: new_sha })
    }

    async fn tag_commit(&self, tag_name: &str, sha: &str) -> Result<()> {
        let url = self.endpoint("refs")?;
        let body = vec![RefUpdate {
            name: format!("refs/tags/{tag_name}"),
            old_object_id: ZERO_SHA.to_string(),
            new_object_id: Some(sha.to_string()),
        }];
        let response = self.client.post(url).json(&body).send().await?;
        response.error_for_status()?;
        Ok(())
    }

    async fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        let candidates =
            self.find_pr_by_branch("active", &req.head_branch).await?;
        let target = format!("refs/heads/{}", req.base_branch);
        let mut matches = vec![];
        for pr in candidates.into_iter() {
            if pr.target_ref_name != target {
                continue;
            }
            // Match by either current or legacy pending label.
            let pending =
                self.pr_has_label(pr.pull_request_id, PENDING_LABEL).await?
                    || self
                        .pr_has_label(pr.pull_request_id, LEGACY_PENDING_LABEL)
                        .await?;
            if !pending {
                continue;
            }
            matches.push(pr);
        }
        if matches.len() > 1 {
            return Err(ReleasaurusError::forge(format!(
                "Found more than one open release PR with pending label for branch {}",
                req.head_branch
            )));
        }
        let Some(pr) = matches.pop() else {
            return Ok(None);
        };
        let full = self.get_pr_by_id(pr.pull_request_id).await?;
        Ok(Some(PullRequest {
            number: pr.pull_request_id,
            sha: pr
                .last_merge_source_commit
                .map(|c| c.commit_id)
                .unwrap_or_default(),
            body: full.description.unwrap_or_default(),
        }))
    }

    async fn get_merged_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<PullRequest>> {
        let candidates = self
            .find_pr_by_branch("completed", &req.head_branch)
            .await?;
        let target = format!("refs/heads/{}", req.base_branch);
        let mut matches = vec![];
        for pr in candidates.into_iter() {
            if pr.target_ref_name != target {
                continue;
            }
            let pending =
                self.pr_has_label(pr.pull_request_id, PENDING_LABEL).await?
                    || self
                        .pr_has_label(pr.pull_request_id, LEGACY_PENDING_LABEL)
                        .await?;
            if !pending {
                continue;
            }
            matches.push(pr);
        }
        if matches.len() > 1 {
            return Err(ReleasaurusError::forge(format!(
                "Found more than one closed release PR with pending label for branch {}. \
              You must remove the {PENDING_LABEL} label from all closed release PRs except for the most recent.",
                req.head_branch
            )));
        }
        let Some(pr) = matches.pop() else {
            return Ok(None);
        };
        let sha = pr
            .last_merge_commit
            .as_ref()
            .map(|c| c.commit_id.clone())
            .ok_or_else(|| {
                ReleasaurusError::forge(format!(
                    "no merge commit found for pr {}",
                    pr.pull_request_id
                ))
            })?;
        let full = self.get_pr_by_id(pr.pull_request_id).await?;
        Ok(Some(PullRequest {
            number: pr.pull_request_id,
            sha,
            body: full.description.unwrap_or_default(),
        }))
    }

    async fn create_pr(&self, req: CreatePrRequest) -> Result<PullRequest> {
        let body = CreatePullRequest {
            source_ref_name: format!("refs/heads/{}", req.head_branch),
            target_ref_name: format!("refs/heads/{}", req.base_branch),
            title: req.title,
            description: req.body,
        };
        let url = self.endpoint("pullrequests")?;
        let response = self.client.post(url).json(&body).send().await?;
        let pr: AzurePullRequest = read_json(response).await?;
        Ok(PullRequest {
            number: pr.pull_request_id,
            sha: pr
                .last_merge_source_commit
                .map(|c| c.commit_id)
                .unwrap_or_default(),
            body: pr.description.unwrap_or_default(),
        })
    }

    async fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        let body = UpdatePullRequest {
            title: req.title,
            description: req.body,
        };
        let url = self.endpoint(&format!("pullrequests/{}", req.pr_number))?;
        let response = self.client.patch(url).json(&body).send().await?;
        response.error_for_status()?;
        Ok(())
    }

    async fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        let existing = self.get_pr_labels(req.pr_number).await?;
        let desired: std::collections::HashSet<&str> =
            req.labels.iter().map(|s| s.as_str()).collect();

        for label in existing.iter() {
            if !desired.contains(label.name.as_str()) {
                self.delete_pr_label(req.pr_number, &label.id).await?;
            }
        }

        let existing_names: std::collections::HashSet<&str> =
            existing.iter().map(|l| l.name.as_str()).collect();

        for name in req.labels.iter() {
            if !existing_names.contains(name.as_str()) {
                self.add_pr_label(req.pr_number, name).await?;
            }
        }

        Ok(())
    }

    async fn create_release(
        &self,
        tag: &str,
        _sha: &str,
        _notes: &str,
    ) -> Result<()> {
        info!(
            "azure devops has no native release object — skipping release publish for tag {tag}; \
             changelog commit and tag have already been pushed"
        );
        Ok(())
    }

    fn encode_pr_metadata(&self, json: &str) -> PrMetadataBlock {
        let b64 = BASE64_STANDARD.encode(json.as_bytes());
        PrMetadataBlock {
            inline_content: String::new(),
            div_attribute: format!(r#"data-meta="{b64}""#),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_adds_leading_slash() {
        assert_eq!(normalize_path("Cargo.toml"), "/Cargo.toml");
        assert_eq!(normalize_path("/Cargo.toml"), "/Cargo.toml");
        assert_eq!(normalize_path("./Cargo.toml"), "/Cargo.toml");
        assert_eq!(
            normalize_path("crates/foo/Cargo.toml"),
            "/crates/foo/Cargo.toml"
        );
    }

    #[test]
    fn strip_refs_heads_removes_prefix() {
        assert_eq!(strip_refs_heads("refs/heads/main"), "main");
        assert_eq!(strip_refs_heads("main"), "main");
    }

    #[test]
    fn strip_refs_tags_removes_prefix() {
        assert_eq!(strip_refs_tags("refs/tags/v1.0.0"), "v1.0.0");
        assert_eq!(strip_refs_tags("v1.0.0"), "v1.0.0");
    }

    fn make_jwt(header_json: &str, payload_json: &str) -> String {
        let h = BASE64_URL_SAFE_NO_PAD.encode(header_json.as_bytes());
        let p = BASE64_URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        format!("{h}.{p}.signature")
    }

    #[test]
    fn looks_like_jwt_accepts_signed_token() {
        let token =
            make_jwt(r#"{"alg":"RS256","typ":"JWT"}"#, r#"{"sub":"x"}"#);
        assert!(looks_like_jwt(&token));
    }

    #[test]
    fn looks_like_jwt_rejects_pat() {
        // Azure DevOps PATs are ~52 alphanumeric chars, no dots.
        assert!(!looks_like_jwt(
            "abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrst"
        ));
    }

    #[test]
    fn looks_like_jwt_rejects_wrong_segment_count() {
        let two_parts = "eyJhbGciOiJIUzI1NiJ9.payload";
        assert!(!looks_like_jwt(two_parts));
        let four_parts = "eyJhbGciOiJIUzI1NiJ9.a.b.c";
        assert!(!looks_like_jwt(four_parts));
    }

    #[test]
    fn looks_like_jwt_rejects_header_without_alg() {
        let token = make_jwt(r#"{"typ":"JWT"}"#, r#"{}"#);
        assert!(!looks_like_jwt(&token));
    }

    #[test]
    fn looks_like_jwt_rejects_non_eyj_prefix() {
        // Three dot-separated parts but header doesn't base64url-decode to
        // a JSON object — fast-path rejection.
        assert!(!looks_like_jwt("foo.bar.baz"));
    }

    #[test]
    fn looks_like_jwt_rejects_invalid_base64_header() {
        // Starts with "eyJ" but contains an illegal base64url character.
        assert!(!looks_like_jwt("eyJ!!!.payload.sig"));
    }
}
