//! Implements the Forge trait for Gitlab
use color_eyre::eyre::eyre;
use gitlab::{
    Gitlab as GitlabClient,
    api::{
        Query, ignore,
        merge_requests::MergeRequestState,
        projects::{
            labels::{CreateLabel, Labels},
            merge_requests::{
                CreateMergeRequest, EditMergeRequest, MergeRequests,
            },
            releases::CreateRelease,
            repository::tags::{Tags, TagsOrderBy},
        },
    },
};
use log::*;
use regex::Regex;
use secrecy::ExposeSecret;
use serde::Deserialize;

use crate::{
    analyzer::types::Tag,
    forge::{
        config::{DEFAULT_LABEL_COLOR, PENDING_LABEL, RemoteConfig},
        traits::Forge,
        types::{
            CreatePrRequest, PrLabelsRequest, ReleasePullRequest,
            UpdatePrRequest,
        },
    },
    result::Result,
};

#[derive(Debug, Deserialize)]
struct MergeRequestInfo {
    iid: u64,
    sha: String,
    merged_at: Option<String>,
    merge_commit_sha: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LabelInfo {
    name: String,
}

/// Information about a commit associated with a release.
#[derive(Debug, Deserialize)]
pub struct GitlabCommit {
    pub id: String,
    // pub title: String,
    // pub message: String,
    // pub author_name: String,
    // pub author_email: String,
    // pub created_at: String,
}

// /// Represents a GitLab project release.
// #[derive(Debug, Deserialize)]
// pub struct GitlabRelease {
//     pub tag_name: String,
//     pub description: Option<String>,
// }

/// Represents a Gitlab project Tag
#[derive(Debug, Deserialize)]
pub struct GitlabTag {
    pub name: String,
    pub commit: GitlabCommit,
    // pub message: String,
    // pub target: String,
    // pub release: GitlabRelease,
}

pub struct Gitlab {
    config: RemoteConfig,
    gl: GitlabClient,
    project_id: String,
}

impl Gitlab {
    pub fn new(config: RemoteConfig) -> Result<Self> {
        let project_id = config.path.clone();

        let token = config.token.expose_secret();

        let gl =
            gitlab::GitlabBuilder::new(config.host.clone(), token).build()?;

        Ok(Self {
            config,
            gl,
            project_id,
        })
    }

    fn get_repo_labels(&self) -> Result<Vec<LabelInfo>> {
        let endpoint = Labels::builder().project(&self.project_id).build()?;

        let labels: Vec<LabelInfo> = endpoint.query(&self.gl)?;

        Ok(labels)
    }

    fn create_label(&self, label_name: String) -> Result<LabelInfo> {
        let endpoint = CreateLabel::builder()
            .project(&self.project_id)
            .name(label_name)
            .color(format!("#{}", DEFAULT_LABEL_COLOR))
            .description("".to_string())
            .build()?;

        let label: LabelInfo = endpoint.query(&self.gl)?;

        Ok(label)
    }
}

impl Forge for Gitlab {
    fn config(&self) -> &RemoteConfig {
        &self.config
    }

    /// Get the latest release for the project
    fn get_latest_tag_for_prefix(&self, prefix: &str) -> Result<Option<Tag>> {
        let re = Regex::new(format!(r"^{prefix}").as_str())?;
        let endpoint = Tags::builder()
            .project(&self.project_id)
            .order_by(TagsOrderBy::Updated)
            .build()?;
        let tags: Vec<GitlabTag> = endpoint.query(&self.gl)?;
        for t in tags.into_iter() {
            if re.is_match(&t.name) {
                let stripped = re.replace_all(&t.name, "").to_string();
                if let Ok(sver) = semver::Version::parse(&stripped) {
                    return Ok(Some(Tag {
                        name: t.name,
                        semver: sver,
                        sha: t.commit.id,
                    }));
                }
            }
        }
        Ok(None)
    }

    fn get_open_release_pr(
        &self,
        req: super::types::GetPrRequest,
    ) -> Result<Option<ReleasePullRequest>> {
        // Create the merge requests query to find open MRs
        // targeting the base branch
        let endpoint = MergeRequests::builder()
            .project(&self.project_id)
            .state(MergeRequestState::Opened)
            .source_branch(&req.head_branch)
            .target_branch(&req.base_branch)
            .build()?;

        // Execute the query to get matching merge requests
        let result: std::result::Result<
            Vec<MergeRequestInfo>,
            gitlab::api::ApiError<gitlab::RestError>,
        > = endpoint.query(&self.gl);

        // Execute the query to get matching merge requests
        match result {
            Ok(merge_requests) => {
                // Return the first matching merge request's IID
                // (should only be one for a given branch)
                let first = merge_requests.first();

                if first.is_none() {
                    return Ok(None);
                }

                let merge_request = first.unwrap();

                Ok(Some(ReleasePullRequest {
                    number: merge_request.iid,
                    sha: merge_request.sha.clone(),
                }))
            }
            Err(gitlab::api::ApiError::GitlabWithStatus { status, msg }) => {
                if status == reqwest::StatusCode::NOT_FOUND {
                    Ok(None)
                } else {
                    let msg = format!(
                        "request for pull request failed: status {status}, msg: {msg}"
                    );
                    error!("{msg}");
                    Err(eyre!(msg))
                }
            }
            Err(err) => Err(eyre!(
                "encountered error querying gitlab for merge request: {err}"
            )),
        }
    }

    fn get_merged_release_pr(&self) -> Result<Option<ReleasePullRequest>> {
        info!("looking for closed release prs with pending label");

        // Search for closed merge requests with the pending label
        let endpoint = MergeRequests::builder()
            .project(&self.project_id)
            .state(MergeRequestState::Merged)
            .labels(vec![PENDING_LABEL])
            .build()?;

        let merge_requests: Vec<MergeRequestInfo> = endpoint.query(&self.gl)?;

        if merge_requests.is_empty() {
            warn!(
                "No merged release PRs with the label {} found. Nothing to release",
                PENDING_LABEL
            );
            return Ok(None);
        }

        if merge_requests.len() > 1 {
            return Err(eyre!(
                "Found more than one closed release PR with pending label. \
                This means either release PRs were closed manually or releasaurus failed to remove tags. \
                You must remove the {} label from all closed release PRs except for the most recent.",
                PENDING_LABEL
            ));
        }

        let merge_request = &merge_requests[0];
        info!("found release pr: {}", merge_request.iid);

        // Check if the MR is actually merged (has merged_at timestamp)
        if merge_request.merged_at.is_none() {
            return Err(eyre!(
                "found release PR {} but it hasn't been merged yet",
                merge_request.iid
            ));
        }

        let sha = merge_request
            .merge_commit_sha
            .as_ref()
            .ok_or_else(|| eyre!("no merge_commit_sha found for pr"))?
            .clone();

        Ok(Some(ReleasePullRequest {
            number: merge_request.iid,
            sha,
        }))
    }

    fn create_pr(&self, req: CreatePrRequest) -> Result<ReleasePullRequest> {
        // Create the merge request
        let endpoint = CreateMergeRequest::builder()
            .project(&self.project_id)
            .source_branch(&req.head_branch)
            .target_branch(&req.base_branch)
            .title(&req.title)
            .description(&req.body)
            .build()?;

        // Execute the creation
        let merge_request: MergeRequestInfo = endpoint.query(&self.gl)?;

        Ok(ReleasePullRequest {
            number: merge_request.iid,
            sha: merge_request.sha.clone(),
        })
    }

    fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        // Update the merge request
        let endpoint = EditMergeRequest::builder()
            .project(&self.project_id)
            .merge_request(req.pr_number)
            .title(&req.title)
            .description(&req.body)
            .build()?;

        // Execute the update using ignore since we don't need the response
        ignore(endpoint).query(&self.gl)?;

        Ok(())
    }

    fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        let all_labels = self.get_repo_labels()?;

        let mut labels = vec![];

        for name in req.labels {
            if let Some(label) = all_labels.iter().find(|l| l.name == name) {
                labels.push(label.name.clone());
            } else {
                let label = self.create_label(name)?;
                labels.push(label.name);
            }
        }

        // Update the merge request with combined labels
        let endpoint = EditMergeRequest::builder()
            .project(&self.project_id)
            .merge_request(req.pr_number)
            .labels(labels.iter())
            .build()?;

        // Execute the update
        ignore(endpoint).query(&self.gl)?;

        Ok(())
    }

    fn create_release(&self, tag: &str, sha: &str, notes: &str) -> Result<()> {
        // Create the release
        let endpoint = CreateRelease::builder()
            .project(&self.project_id)
            .tag_name(tag)
            .name(tag)
            .description(notes)
            .ref_sha(sha)
            .build()?;

        // Execute the creation using ignore since we don't need the response
        ignore(endpoint).query(&self.gl)?;

        Ok(())
    }
}
