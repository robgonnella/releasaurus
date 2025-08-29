use color_eyre::eyre::{Result as EyreResult, eyre};
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
use log::*;
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
    config: RemoteConfig,
    gl: GitlabClient,
    project_id: String,
}

impl Gitlab {
    pub fn new(config: RemoteConfig) -> EyreResult<Self> {
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

    fn get_repo_labels(&self) -> EyreResult<Vec<LabelInfo>> {
        let endpoint = Labels::builder().project(&self.project_id).build()?;

        let labels: Vec<LabelInfo> = endpoint.query(&self.gl)?;

        Ok(labels)
    }

    fn create_label(&self, label_name: String) -> EyreResult<LabelInfo> {
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

    fn get_pr_number(
        &self,
        req: super::types::GetPrRequest,
    ) -> EyreResult<Option<u64>> {
        // Create the merge requests query to find open MRs
        // targeting the base branch
        let endpoint = MergeRequests::builder()
            .project(&self.project_id)
            .state(MergeRequestState::Opened)
            .source_branch(&req.head_branch)
            .target_branch(&req.base_branch)
            .build()?;

        // Execute the query to get matching merge requests
        let result: Result<
            Vec<MergeRequestInfo>,
            gitlab::api::ApiError<gitlab::RestError>,
        > = endpoint.query(&self.gl);

        // Execute the query to get matching merge requests
        match result {
            Ok(merge_requests) => {
                // Return the first matching merge request's IID
                // (should only be one for a given branch)
                let result = merge_requests.first().map(|mr| mr.iid);
                Ok(result)
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

    fn create_pr(&self, req: CreatePrRequest) -> EyreResult<u64> {
        // Create the merge request
        let endpoint = CreateMergeRequest::builder()
            .project(&self.project_id)
            .source_branch(&req.head_branch)
            .target_branch(&req.base_branch)
            .title(&req.title)
            .description(&req.body)
            .build()?;

        // Execute the creation
        let response: MergeRequestInfo = endpoint.query(&self.gl)?;

        Ok(response.iid)
    }

    fn update_pr(&self, req: UpdatePrRequest) -> EyreResult<()> {
        // Update the merge request
        let endpoint = EditMergeRequest::builder()
            .project(&self.project_id)
            .merge_request(req.pr_number)
            .description(&req.body)
            .build()?;

        // Execute the update using ignore since we don't need the response
        ignore(endpoint).query(&self.gl)?;

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
            .build()?;

        // Execute the update
        ignore(endpoint).query(&self.gl)?;

        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "forge-tests")]
mod tests {
    use color_eyre::eyre::Result;
    use gitlab::{
        Gitlab as GitlabClient,
        api::{
            Query, ignore,
            projects::{
                labels::DeleteLabel,
                merge_requests::{EditMergeRequest, MergeRequestStateEvent},
            },
        },
    };
    use secrecy::{ExposeSecret, SecretString};
    use std::env;

    use crate::forge::{
        config::{PENDING_LABEL, RemoteConfig},
        gitlab::Gitlab,
        traits::Forge,
        types::{
            CreatePrRequest, GetPrRequest, PrLabelsRequest, UpdatePrRequest,
        },
    };

    fn delete_label(
        gl: &GitlabClient,
        project_id: String,
        label: String,
    ) -> Result<()> {
        let endpoint = DeleteLabel::builder()
            .project(project_id)
            .label(label)
            .build()?;
        ignore(endpoint).query(gl)?;
        Ok(())
    }

    fn close_pr(
        gl: &GitlabClient,
        project_id: String,
        pr_number: u64,
    ) -> Result<()> {
        let endpoint = EditMergeRequest::builder()
            .project(project_id)
            .merge_request(pr_number)
            .state_event(MergeRequestStateEvent::Close)
            .build()?;

        ignore(endpoint).query(gl)?;

        Ok(())
    }

    #[test]
    fn test_gitlab_forge() {
        let result = env::var("GL_TEST_TOKEN");
        assert!(
            result.is_ok(),
            "must set GL_TEST_TOKEN as environment variable to run these tests"
        );

        let token = result.unwrap();

        let remote_config = RemoteConfig {
            scheme: "https".into(),
            host: "gitlab.com".into(),
            owner: "rgon".into(),
            repo: "test-repo".into(),
            path: format!("{}/{}", "rgon", "test-repo"),
            token: SecretString::from(token),
            commit_link_base_url: "".into(),
            release_link_base_url: "".into(),
        };

        let result = gitlab::GitlabBuilder::new(
            remote_config.host.clone(),
            remote_config.token.expose_secret(),
        )
        .build();
        assert!(result.is_ok(), "failed to create gitlab client");
        let gl = result.unwrap();
        let project_id =
            format!("{}/{}", remote_config.owner, remote_config.repo);

        let result = Gitlab::new(remote_config.clone());
        assert!(result.is_ok(), "failed to create gitlab forge");
        let forge = result.unwrap();

        let req = CreatePrRequest {
            head_branch: "test-branch".into(),
            base_branch: "main".into(),
            body: "super duper!".into(),
            title: "The is my test PR".into(),
        };

        let result = forge.create_pr(req);
        assert!(result.is_ok(), "failed to create PR");
        let pr_number = result.unwrap();

        let req = UpdatePrRequest {
            pr_number,
            body: "now this is a good body!".into(),
        };

        let result = forge.update_pr(req);
        assert!(result.is_ok(), "failed to update PR");

        let new_label = "releasaurus:1".to_string();

        let req = PrLabelsRequest {
            pr_number,
            labels: vec![new_label.clone(), PENDING_LABEL.into()],
        };

        let result = forge.replace_pr_labels(req);
        assert!(result.is_ok(), "failed to replace PR labels");

        let req = GetPrRequest {
            head_branch: "test-branch".into(),
            base_branch: "main".into(),
        };
        let result = forge.get_pr_number(req);
        assert!(result.is_ok(), "failed to get PR number");

        let result = close_pr(&gl, project_id.clone(), pr_number);
        assert!(result.is_ok(), "failed to close PR");

        let result = delete_label(&gl, project_id, new_label);
        assert!(result.is_ok(), "failed to delete label")
    }
}
