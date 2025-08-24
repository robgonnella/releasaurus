use color_eyre::eyre::{Context, Result};
use gitlab::{
    Gitlab as GitlabClient,
    api::{
        Query, ignore,
        merge_requests::MergeRequestState,
        projects::merge_requests::{
            CreateMergeRequest, EditMergeRequest, MergeRequest, MergeRequests,
        },
    },
};
use secrecy::ExposeSecret;
use serde::Deserialize;

use crate::{config::RemoteConfig, forge::traits::Forge};

pub struct Gitlab {
    gl: GitlabClient,
    project_id: String,
}

impl Gitlab {
    pub fn new(config: RemoteConfig) -> Result<Self> {
        let project_id = format!("{}/{}", config.owner, config.repo);

        let token = config.token.expose_secret();

        let gl =
            gitlab::GitlabBuilder::new(config.host.clone(), token).build()?;

        Ok(Self { gl, project_id })
    }

    fn get_merge_request_by_iid(&self, iid: u64) -> Result<MergeRequestInfo> {
        let get_endpoint = MergeRequest::builder()
            .project(self.project_id.clone())
            .merge_request(iid)
            .build()
            .wrap_err("failed to build get merge request")?;

        let mr: MergeRequestInfo = get_endpoint
            .query(&self.gl)
            .wrap_err("failed to get current merge request")?;

        Ok(mr)
    }
}

// Simple structs to deserialize GitLab API responses
#[derive(Debug, Deserialize)]
struct MergeRequestInfo {
    iid: u64,
    labels: Option<Vec<String>>,
}

impl Forge for Gitlab {
    fn get_pr_number(
        &self,
        req: super::types::GetPrRequest,
    ) -> color_eyre::eyre::Result<Option<u64>> {
        // Create the merge requests query to find open MRs
        // targeting the base branch
        let endpoint = MergeRequests::builder()
            .project(self.project_id.clone())
            .state(MergeRequestState::Opened)
            .source_branch(&req.head_branch)
            .target_branch(&req.base_branch)
            .build()
            .wrap_err("failed to build merge request query")?;

        // Execute the query to get matching merge requests
        let merge_requests: Vec<MergeRequestInfo> = endpoint
            .query(&self.gl)
            .wrap_err("failed to query merge requests")?;

        // Return the first matching merge request's IID
        // (should only be one for a given branch)
        Ok(merge_requests.first().map(|mr| mr.iid))
    }

    fn create_pr(
        &self,
        req: super::types::CreatePrRequest,
    ) -> color_eyre::eyre::Result<u64> {
        // Create the merge request
        let endpoint = CreateMergeRequest::builder()
            .project(self.project_id.clone())
            .source_branch(&req.head_branch)
            .target_branch(&req.base_branch)
            .title(&req.title)
            .description(&req.body)
            .build()
            .wrap_err("failed to build create merge request")?;

        // Execute the creation
        let response: MergeRequestInfo = endpoint
            .query(&self.gl)
            .wrap_err("Failed to create merge request")?;

        Ok(response.iid)
    }

    fn update_pr(
        &self,
        req: super::types::UpdatePrRequest,
    ) -> color_eyre::eyre::Result<()> {
        // Update the merge request
        let endpoint = EditMergeRequest::builder()
            .project(self.project_id.clone())
            .merge_request(req.pr_number)
            .description(&req.body)
            .build()
            .wrap_err("failed to build edit merge request")?;

        // Execute the update using ignore since we don't need the response
        ignore(endpoint)
            .query(&self.gl)
            .wrap_err("Failed to update merge request")?;

        Ok(())
    }

    fn add_pr_labels(
        &self,
        req: super::types::PrLabelsRequest,
    ) -> color_eyre::eyre::Result<()> {
        // First, get the current merge request to see existing labels
        let current_mr = self.get_merge_request_by_iid(req.pr_number)?;

        // Combine existing labels with new ones
        let mut all_labels = current_mr.labels.unwrap_or_default();
        for label in &req.labels {
            if !all_labels.contains(label) {
                all_labels.push(label.clone());
            }
        }

        // Update the merge request with combined labels
        let endpoint = EditMergeRequest::builder()
            .project(self.project_id.clone())
            .merge_request(req.pr_number)
            .labels(all_labels.iter().map(|s| s.as_str()))
            .build()
            .wrap_err("failed to build edit merge request for labels")?;

        // Execute the update
        ignore(endpoint)
            .query(&self.gl)
            .wrap_err("failed to add labels to merge request")?;

        Ok(())
    }

    fn remove_pr_labels(
        &self,
        req: super::types::PrLabelsRequest,
    ) -> color_eyre::eyre::Result<()> {
        // First, get the current merge request to see existing labels
        let current_mr = self.get_merge_request_by_iid(req.pr_number)?;

        // Remove specified labels from the existing labels
        let remaining_labels: Vec<String> = current_mr
            .labels
            .unwrap_or_default()
            .into_iter()
            .filter(|label| !req.labels.contains(label))
            .collect();

        // Update the merge request with the remaining labels
        let endpoint = EditMergeRequest::builder()
            .project(self.project_id.clone())
            .merge_request(req.pr_number)
            .labels(remaining_labels.iter().map(|s| s.as_str()))
            .build()
            .wrap_err("failed to build edit merge request for label removal")?;

        // Execute the update
        ignore(endpoint)
            .query(&self.gl)
            .wrap_err("failed to remove labels from merge request")?;

        Ok(())
    }
}
