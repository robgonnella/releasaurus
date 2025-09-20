//! Implements the Forge trait for Gitea
use color_eyre::eyre::eyre;
use log::*;
use regex::Regex;
use reqwest::{
    Url,
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

use crate::{
    analyzer::types::Tag,
    forge::{
        config::{DEFAULT_LABEL_COLOR, PENDING_LABEL, RemoteConfig},
        traits::Forge,
        types::{
            CreatePrRequest, GetPrRequest, PrLabelsRequest, ReleasePullRequest,
            UpdatePrRequest,
        },
    },
    result::Result,
};

#[derive(Debug, Default, Serialize)]
struct CreateLabel {
    name: String,
    color: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: u64,
    pub name: String,
    color: String,
    description: String,
    exclusive: bool,
    is_archived: bool,
}

#[derive(Debug, Deserialize)]
struct PullRequestHead {
    sha: String,
}

#[derive(Debug, Deserialize)]
struct PullRequest {
    number: u64,
    head: PullRequestHead,
    merged: Option<bool>,
    merge_commit_sha: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Issue {
    number: u64,
}

#[derive(Debug, Serialize)]
struct CreatePull {
    title: String,
    body: String,
    head: String,
    base: String,
}

#[derive(Debug, Serialize)]
struct UpdatePullBody {
    title: String,
    body: String,
}

#[derive(Debug, Serialize)]
struct UpdatePullLabels {
    labels: Vec<u64>,
}

#[derive(Debug, Serialize)]
struct CreateRelease {
    tag_name: String,
    target_commitish: String,
    name: String,
    body: String,
    draft: bool,
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct GiteaCommit {
    sha: String,
}

#[derive(Debug, Deserialize)]
struct GiteaTag {
    name: String,
    commit: GiteaCommit,
    // id: String,
    // message: String,
}

pub struct Gitea {
    config: RemoteConfig,
    base_url: Url,
    client: Client,
}

impl Gitea {
    pub fn new(config: RemoteConfig) -> Result<Self> {
        let token = config.token.expose_secret();

        let mut headers = HeaderMap::new();

        let token_value =
            HeaderValue::from_str(format!("token {}", token).as_str())?;

        headers.append("Authorization", token_value);

        let client = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .build()?;

        let base_url = Url::parse(
            format!(
                "{}://{}/api/v1/repos/{}/",
                config.scheme, config.host, config.path
            )
            .as_str(),
        )?;

        Ok(Gitea {
            config,
            client,
            base_url,
        })
    }

    fn get_all_labels(&self) -> Result<Vec<Label>> {
        let labels_url = self.base_url.join("labels")?;
        let request = self.client.get(labels_url).build()?;
        let response = self.client.execute(request)?;
        let result = response.error_for_status()?;
        let labels: Vec<Label> = result.json()?;
        Ok(labels)
    }

    fn create_label(&self, label_name: String) -> Result<Label> {
        let labels_url = self.base_url.join("labels")?;
        let request = self
            .client
            .post(labels_url)
            .json(&CreateLabel {
                name: label_name,
                color: DEFAULT_LABEL_COLOR.to_string(),
            })
            .build()?;
        let response = self.client.execute(request)?;
        let result = response.error_for_status()?;
        let label: Label = result.json()?;
        Ok(label)
    }
}

impl Forge for Gitea {
    fn config(&self) -> &RemoteConfig {
        &self.config
    }

    fn get_latest_tag_for_prefix(&self, prefix: &str) -> Result<Option<Tag>> {
        let re = Regex::new(format!(r"^{prefix}").as_str())?;

        // Search for open issues with the pending label
        let tags_url = self.base_url.join("tags")?;
        let request = self.client.get(tags_url).build()?;
        let response = self.client.execute(request)?;
        let result = response.error_for_status()?;
        let tags: Vec<GiteaTag> = result.json()?;

        for tag in tags.into_iter() {
            if re.is_match(&tag.name) {
                let stripped = re.replace_all(&tag.name, "").to_string();
                if let Ok(sver) = semver::Version::parse(&stripped) {
                    return Ok(Some(Tag {
                        name: tag.name,
                        semver: sver,
                        sha: tag.commit.sha,
                    }));
                }
            }
        }

        Ok(None)
    }

    fn get_open_release_pr(
        &self,
        _req: GetPrRequest,
    ) -> Result<Option<ReleasePullRequest>> {
        info!("looking for open release prs with pending label");

        // Search for open issues with the pending label
        let issues_url = self.base_url.join(&format!(
            "issues?state=open&type=pulls&labels={}&page=1&limit=2",
            PENDING_LABEL
        ))?;

        let request = self.client.get(issues_url).build()?;
        let response = self.client.execute(request)?;
        let result = response.error_for_status()?;
        let issues: Vec<Issue> = result.json()?;

        if issues.is_empty() {
            return Ok(None);
        }

        if issues.len() > 1 {
            return Err(eyre!(
                "Found more than one open release PR with pending label: {}. \
                This means either releasaurus incorrectly created more than one open PR, or \
                the pending label was manually added to another PR and must be removed",
                PENDING_LABEL
            ));
        }

        let issue = &issues[0];
        info!("found open release pr: {}", issue.number);

        // Get the full PR details
        let pr_url = self.base_url.join(&format!("pulls/{}", issue.number))?;
        let request = self.client.get(pr_url).build()?;
        let response = self.client.execute(request)?;
        let result = response.error_for_status()?;
        let pr: PullRequest = result.json()?;

        // Make sure the PR isn't merged
        if let Some(merged) = pr.merged
            && merged
        {
            return Err(eyre!(
                "found release PR {} but it has already been merged",
                pr.number
            ));
        }

        let sha = pr.head.sha;

        Ok(Some(ReleasePullRequest {
            number: pr.number,
            sha,
        }))
    }

    fn get_merged_release_pr(&self) -> Result<Option<ReleasePullRequest>> {
        info!("looking for closed release prs with pending label");

        // Search for closed issues with the pending label
        let issues_url = self.base_url.join(&format!(
            "issues?state=closed&labels={}&page=1&limit=2",
            PENDING_LABEL
        ))?;

        let request = self.client.get(issues_url).build()?;
        let response = self.client.execute(request)?;
        let result = response.error_for_status()?;
        let issues: Vec<Issue> = result.json()?;

        if issues.is_empty() {
            warn!(
                "No merged release PRs with the label {} found. Nothing to release",
                PENDING_LABEL
            );
            return Ok(None);
        }

        if issues.len() > 1 {
            return Err(eyre!(
                "Found more than one closed release PR with pending label. \
                This means either release PRs were closed manually or releasaurus failed to remove tags. \
                You must remove the {} label from all closed release PRs except for the most recent.",
                PENDING_LABEL
            ));
        }

        let issue = &issues[0];
        info!("found release pr: {}", issue.number);

        // Get the full PR details
        let pr_url = self.base_url.join(&format!("pulls/{}", issue.number))?;
        let request = self.client.get(pr_url).build()?;
        let response = self.client.execute(request)?;
        let result = response.error_for_status()?;
        let pr: PullRequest = result.json()?;

        // Check if the PR is actually merged
        if let Some(merged) = pr.merged
            && !merged
        {
            return Err(eyre!(
                "found release PR {} but it hasn't been merged yet",
                pr.number
            ));
        }

        let sha = pr
            .merge_commit_sha
            .ok_or_else(|| eyre!("no merge_commit_sha found for pr"))?;

        Ok(Some(ReleasePullRequest {
            number: pr.number,
            sha,
        }))
    }

    fn create_pr(&self, req: CreatePrRequest) -> Result<ReleasePullRequest> {
        let data = CreatePull {
            title: req.title,
            body: req.body,
            head: req.head_branch,
            base: req.base_branch,
        };
        let pulls_url = self.base_url.join("pulls")?;
        let request = self.client.post(pulls_url).json(&data).build()?;
        let response = self.client.execute(request)?;
        let result = response.error_for_status()?;
        let pr: PullRequest = result.json()?;

        Ok(ReleasePullRequest {
            number: pr.number,
            sha: pr.head.sha,
        })
    }

    fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        let data = UpdatePullBody {
            title: req.title,
            body: req.body,
        };
        let pulls_url = self
            .base_url
            .join(format!("pulls/{}", req.pr_number).as_str())?;
        let request = self.client.patch(pulls_url).json(&data).build()?;
        let response = self.client.execute(request)?;
        response.error_for_status()?;
        Ok(())
    }

    fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        let all_labels = self.get_all_labels()?;

        let mut labels = vec![];

        for name in req.labels {
            if let Some(label) = all_labels.iter().find(|l| l.name == name) {
                labels.push(label.id);
            } else {
                let label = self.create_label(name)?;
                labels.push(label.id);
            }
        }

        let data = UpdatePullLabels { labels };

        let labels_url = self
            .base_url
            .join(format!("issues/{}/labels", req.pr_number).as_str())?;

        let request = self.client.put(labels_url).json(&data).build()?;
        let response = self.client.execute(request)?;
        response.error_for_status()?;

        Ok(())
    }

    fn create_release(&self, tag: &str, sha: &str, notes: &str) -> Result<()> {
        let data = CreateRelease {
            tag_name: tag.to_string(),
            target_commitish: sha.to_string(),
            name: tag.to_string(),
            body: notes.to_string(),
            draft: false,
            prerelease: false,
        };

        let releases_url = self.base_url.join("releases")?;
        let request = self.client.post(releases_url).json(&data).build()?;
        let response = self.client.execute(request)?;
        response.error_for_status()?;

        Ok(())
    }
}
