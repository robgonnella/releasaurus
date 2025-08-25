use color_eyre::eyre::{Context, Result};
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
        },
    },
};
use secrecy::ExposeSecret;
use serde::Deserialize;

use crate::forge::{
    config::{DEFAULT_LABEL_COLOR, RemoteConfig},
    traits::Forge,
    types::{CreatePrRequest, PrLabelsRequest, UpdatePrRequest},
};

#[derive(Debug, Deserialize)]
struct MergeRequestInfo {
    iid: u64,
}

#[derive(Debug, Deserialize)]
struct LabelInfo {
    name: String,
}

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

    fn get_repo_labels(&self) -> Result<Vec<LabelInfo>> {
        let endpoint = Labels::builder()
            .project(&self.project_id)
            .build()
            .wrap_err("failed to build labels request")?;

        let labels: Vec<LabelInfo> = endpoint
            .query(&self.gl)
            .wrap_err("failed to query project labels")?;

        Ok(labels)
    }

    fn create_label(&self, label_name: String) -> Result<LabelInfo> {
        let endpoint = CreateLabel::builder()
            .name(label_name)
            .color(DEFAULT_LABEL_COLOR)
            .description("".to_string())
            .build()
            .wrap_err("failed to build label endpoint")?;

        let label: LabelInfo = endpoint
            .query(&self.gl)
            .wrap_err("failed to create label")?;

        Ok(label)
    }
}

impl Forge for Gitlab {
    fn get_pr_number(
        &self,
        req: super::types::GetPrRequest,
    ) -> color_eyre::eyre::Result<Option<u64>> {
        // Create the merge requests query to find open MRs
        // targeting the base branch
        let endpoint = MergeRequests::builder()
            .project(&self.project_id)
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

    fn create_pr(&self, req: CreatePrRequest) -> color_eyre::eyre::Result<u64> {
        // Create the merge request
        let endpoint = CreateMergeRequest::builder()
            .project(&self.project_id)
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

    fn update_pr(&self, req: UpdatePrRequest) -> color_eyre::eyre::Result<()> {
        // Update the merge request
        let endpoint = EditMergeRequest::builder()
            .project(&self.project_id)
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

    fn replace_pr_labels(
        &self,
        req: PrLabelsRequest,
    ) -> color_eyre::eyre::Result<()> {
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
            .build()
            .wrap_err("failed to build edit merge request for labels")?;

        // Execute the update
        ignore(endpoint)
            .query(&self.gl)
            .wrap_err("failed to add labels to merge request")?;

        Ok(())
    }
}
