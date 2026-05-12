use async_trait::async_trait;
use base64::{Engine, prelude::BASE64_STANDARD};
use color_eyre::eyre::Result;
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use serde::Deserialize;
use serde_json::json;
use url::Url;

use crate::forge::{config::RepoUrl, tests::common::traits::ForgeTestHelper};

const API_VERSION: &str = "7.1";
const LABELS_API_VERSION: &str = "7.1-preview.1";
const ZERO_SHA: &str = "0000000000000000000000000000000000000000";
const PENDING_LABELS: &[&str] =
    &["releasaurus:pending", "releasaurus::pending"];

#[derive(Debug, Deserialize)]
struct AzureRepo {
    #[serde(rename = "defaultBranch")]
    default_branch: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AzureRef {
    name: String,
    #[serde(rename = "objectId")]
    object_id: String,
}

#[derive(Debug, Deserialize)]
struct AzureList<T> {
    #[serde(default = "Vec::new")]
    value: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct AzurePr {
    #[serde(rename = "pullRequestId")]
    pull_request_id: u64,
    #[serde(rename = "lastMergeSourceCommit", default)]
    last_merge_source_commit: Option<AzureCommitRef>,
}

#[derive(Debug, Deserialize, Clone)]
struct AzureCommitRef {
    #[serde(rename = "commitId")]
    commit_id: String,
}

#[derive(Debug, Deserialize)]
struct AzureLabel {
    id: String,
    name: String,
}

pub struct AzureDevopsForgeTestHelper {
    client: Client,
    base_url: Url,
    default_branch: String,
    reset_sha: String,
}

impl AzureDevopsForgeTestHelper {
    pub async fn new(repo: &RepoUrl, token: &str, reset_sha: &str) -> Self {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .ok();

        let basic = BASE64_STANDARD.encode(format!(":{}", token).as_bytes());
        let mut headers = HeaderMap::new();
        headers.append(
            "Authorization",
            HeaderValue::from_str(&format!("Basic {}", basic)).unwrap(),
        );
        headers.append("Accept", HeaderValue::from_static("application/json"));

        let client =
            Client::builder().default_headers(headers).build().unwrap();

        let link_base_url = match repo.port {
            Some(port) => {
                format!("{}://{}:{}", repo.scheme, repo.host, port)
            }
            None => format!("{}://{}", repo.scheme, repo.host),
        };

        let base_url = Url::parse(&format!(
            "{}/{}/_apis/git/repositories/{}/",
            link_base_url, repo.owner, repo.name
        ))
        .unwrap();

        let mut metadata_url = base_url.clone();
        metadata_url
            .query_pairs_mut()
            .append_pair("api-version", API_VERSION);
        let response = client.get(metadata_url).send().await.unwrap();
        let result = response.error_for_status().unwrap();
        let meta: AzureRepo = result.json().await.unwrap();
        let default_branch = meta
            .default_branch
            .as_deref()
            .map(|s| s.strip_prefix("refs/heads/").unwrap_or(s).to_string())
            .expect("repository must have a default branch");

        Self {
            client,
            base_url,
            default_branch,
            reset_sha: reset_sha.into(),
        }
    }

    fn endpoint(&self, path: &str) -> Url {
        let mut url = self.base_url.join(path).unwrap();
        url.query_pairs_mut()
            .append_pair("api-version", API_VERSION);
        url
    }

    async fn list_refs(&self, filter: &str) -> Result<Vec<AzureRef>> {
        let mut url = self.base_url.join("refs")?;
        url.query_pairs_mut()
            .append_pair("api-version", API_VERSION)
            .append_pair("filter", filter);
        let response = self.client.get(url).send().await?;
        let result = response.error_for_status()?;
        let list: AzureList<AzureRef> = result.json().await?;
        Ok(list.value)
    }

    async fn delete_refs(&self, refs: &[AzureRef]) -> Result<()> {
        if refs.is_empty() {
            return Ok(());
        }
        let body: Vec<_> = refs
            .iter()
            .map(|r| {
                json!({
                    "name": r.name,
                    "oldObjectId": r.object_id,
                    "newObjectId": ZERO_SHA,
                })
            })
            .collect();
        let url = self.endpoint("refs");
        let response = self.client.post(url).json(&body).send().await?;
        response.error_for_status()?;
        Ok(())
    }

    async fn remove_pending_labels_from_completed_prs(&self) -> Result<()> {
        log::info!("removing pending labels from completed prs");
        let mut url = self.base_url.join("pullrequests")?;
        url.query_pairs_mut()
            .append_pair("api-version", API_VERSION)
            .append_pair("searchCriteria.status", "completed")
            .append_pair("$top", "200");
        let response = self.client.get(url).send().await?;
        let result = response.error_for_status()?;
        let list: AzureList<AzurePr> = result.json().await?;

        for pr in list.value.iter() {
            let mut labels_url = self
                .base_url
                .join(&format!("pullrequests/{}/labels", pr.pull_request_id))?;
            labels_url
                .query_pairs_mut()
                .append_pair("api-version", LABELS_API_VERSION);
            let resp = self.client.get(labels_url).send().await?;
            if !resp.status().is_success() {
                continue;
            }
            let label_list: AzureList<AzureLabel> = resp.json().await?;
            for label in label_list.value.iter() {
                if PENDING_LABELS.contains(&label.name.as_str()) {
                    let mut del_url = self.base_url.join(&format!(
                        "pullrequests/{}/labels/{}",
                        pr.pull_request_id, label.id
                    ))?;
                    del_url
                        .query_pairs_mut()
                        .append_pair("api-version", LABELS_API_VERSION);
                    let _ = self.client.delete(del_url).send().await;
                }
            }
        }
        Ok(())
    }

    async fn abandon_all_active_prs(&self) -> Result<()> {
        log::info!("abandoning all active prs");
        let mut url = self.base_url.join("pullrequests")?;
        url.query_pairs_mut()
            .append_pair("api-version", API_VERSION)
            .append_pair("searchCriteria.status", "active")
            .append_pair("$top", "200");
        let response = self.client.get(url).send().await?;
        let result = response.error_for_status()?;
        let list: AzureList<AzurePr> = result.json().await?;

        for pr in list.value.iter() {
            let pr_url =
                self.endpoint(&format!("pullrequests/{}", pr.pull_request_id));
            let body = json!({ "status": "abandoned" });
            let response = self.client.patch(pr_url).json(&body).send().await?;
            response.error_for_status()?;
        }
        Ok(())
    }

    async fn delete_all_tags(&self) -> Result<()> {
        log::info!("deleting all tags");
        let refs = self.list_refs("tags/").await?;
        self.delete_refs(&refs).await
    }

    async fn delete_all_branches(&self) -> Result<()> {
        log::info!("deleting all branches except default");
        let refs = self.list_refs("heads/").await?;
        let default_ref = format!("refs/heads/{}", self.default_branch);
        let to_delete: Vec<AzureRef> =
            refs.into_iter().filter(|r| r.name != default_ref).collect();
        self.delete_refs(&to_delete).await
    }

    async fn force_reset_history(&self) -> Result<()> {
        log::info!("force resetting history");
        let default_ref = format!("refs/heads/{}", self.default_branch);

        let heads = self
            .list_refs(&format!("heads/{}", self.default_branch))
            .await?;
        let current_head = heads
            .iter()
            .find(|r| r.name == default_ref)
            .map(|r| r.object_id.clone())
            .ok_or_else(|| {
                color_eyre::eyre::eyre!("default branch ref not found")
            })?;

        if current_head == self.reset_sha {
            log::info!("already at reset SHA, nothing to do");
            return Ok(());
        }

        // Direct non-fast-forward ref update — requires the repository
        // to have "Allow rewriting history (force push)" enabled.
        let body = vec![json!({
            "name": default_ref,
            "oldObjectId": current_head,
            "newObjectId": self.reset_sha,
        })];
        let url = self.endpoint("refs");
        let response = self.client.post(url).json(&body).send().await?;
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(color_eyre::eyre::eyre!(
                "force reset refs POST failed: {status} — {body_text}"
            ));
        }
        let result: AzureList<serde_json::Value> =
            serde_json::from_str(&body_text).map_err(|e| {
                color_eyre::eyre::eyre!(
                    "failed to parse refs update response: {e} — {body_text}"
                )
            })?;
        for item in result.value.iter() {
            let success = item
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !success {
                let update_status = item
                    .get("updateStatus")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                return Err(color_eyre::eyre::eyre!(
                    "force reset rejected ({update_status}): enable \
                     'Allow rewriting history' on the default branch in \
                     Azure DevOps repo settings"
                ));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ForgeTestHelper for AzureDevopsForgeTestHelper {
    fn supports_native_releases(&self) -> bool {
        false
    }

    async fn reset(&self) -> Result<()> {
        self.remove_pending_labels_from_completed_prs().await?;
        self.abandon_all_active_prs().await?;
        self.delete_all_tags().await?;
        self.delete_all_branches().await?;
        self.force_reset_history().await
    }

    async fn merge_pr(&self, pr_number: u64) -> Result<()> {
        // Azure requires the caller to assert which source-tip it
        // intends to complete the merge against. Fetch the PR first
        // to grab `lastMergeSourceCommit.commitId`.
        let pr_url = self.endpoint(&format!("pullrequests/{pr_number}"));
        let response = self.client.get(pr_url.clone()).send().await?;
        let result = response.error_for_status()?;
        let pr: AzurePr = result.json().await?;
        let commit_id = pr
            .last_merge_source_commit
            .as_ref()
            .map(|c| c.commit_id.clone())
            .ok_or_else(|| {
                color_eyre::eyre::eyre!(
                    "pr {} has no lastMergeSourceCommit yet — try again",
                    pr_number
                )
            })?;

        let body = json!({
            "status": "completed",
            "lastMergeSourceCommit": {"commitId": commit_id},
            "completionOptions": {
                "mergeStrategy": "noFastForward",
                "deleteSourceBranch": false,
            }
        });
        let response = self.client.patch(pr_url).json(&body).send().await?;
        response.error_for_status()?;
        Ok(())
    }
}
