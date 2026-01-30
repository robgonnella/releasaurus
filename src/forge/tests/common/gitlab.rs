use std::borrow::Cow;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use derive_builder::Builder;
use git_url_parse::GitUrl;
use gitlab::{
    AsyncGitlab,
    api::{
        AsyncQuery, Endpoint, QueryParams,
        common::NameOrId,
        ignore,
        merge_requests::MergeRequestState,
        projects::{
            EditProject, Project,
            merge_requests::{
                EditMergeRequest, MergeMergeRequest, MergeRequestStateEvent,
                MergeRequests,
            },
            releases::ProjectReleases,
            repository::{
                branches::{Branches, CreateBranch, DeleteBranch},
                tags::{DeleteTag, Tags, TagsOrderBy},
            },
        },
    },
};
use reqwest::Method;
use serde::Deserialize;

use crate::forge::tests::common::traits::ForgeTestHelper;

#[derive(Debug, Deserialize)]
struct GitlabTag {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GitlabBranch {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MergeRequestInfo {
    iid: u64,
}

#[derive(Debug, Deserialize)]
struct GitlabRelease {
    tag_name: String,
}

#[derive(Debug, Builder)]
#[builder(setter(strip_option))]
pub struct DeleteMergeRequest<'a> {
    #[builder(setter(into))]
    project: NameOrId<'a>,

    #[builder(setter(into))]
    iid: u64,
}

impl<'a> DeleteMergeRequest<'a> {
    pub fn builder() -> DeleteMergeRequestBuilder<'a> {
        DeleteMergeRequestBuilder::default()
    }
}

impl Endpoint for DeleteMergeRequest<'_> {
    fn method(&self) -> Method {
        Method::DELETE
    }

    fn endpoint(&self) -> Cow<'static, str> {
        format!("projects/{}/merge_requests/{}", self.project, self.iid).into()
    }

    fn parameters(&self) -> QueryParams<'_> {
        QueryParams::default()
    }
}

#[derive(Builder)]
#[builder(setter(strip_option))]
pub struct DeleteRelease<'a> {
    /// The project to query for release.
    #[builder(setter(into))]
    project: NameOrId<'a>,

    /// Gets a release for a specific tag
    #[builder(setter(into))]
    tag_name: Cow<'a, str>,
}

impl<'a> DeleteRelease<'a> {
    /// Create a builder for the endpoint.
    pub fn builder() -> DeleteReleaseBuilder<'a> {
        DeleteReleaseBuilder::default()
    }
}

impl Endpoint for DeleteRelease<'_> {
    fn method(&self) -> Method {
        Method::DELETE
    }

    fn endpoint(&self) -> Cow<'static, str> {
        format!("projects/{}/releases/{}", self.project, self.tag_name).into()
    }

    fn parameters(&self) -> QueryParams<'_> {
        QueryParams::default()
    }
}

pub struct GitlabForgeTestHelper {
    gl: AsyncGitlab,
    project_id: String,
    default_branch: String,
    reset_sha: String,
}

impl GitlabForgeTestHelper {
    pub async fn new(repo: &GitUrl, token: &str, reset_sha: &str) -> Self {
        let host = repo.host.as_ref().unwrap().clone();
        let project_id = repo.fullname.clone();

        let gl = gitlab::GitlabBuilder::new(host, token)
            .build_async()
            .await
            .unwrap();

        let endpoint = Project::builder().project(&project_id).build().unwrap();
        let gl_project: serde_json::Value =
            endpoint.query_async(&gl).await.unwrap();

        let default_branch =
            gl_project["default_branch"].as_str().unwrap().to_string();

        Self {
            gl,
            project_id,
            default_branch,
            reset_sha: reset_sha.into(),
        }
    }

    async fn close_all_prs(&self) -> Result<()> {
        let endpoint = MergeRequests::builder()
            .project(&self.project_id)
            .state(MergeRequestState::Opened)
            .build()?;

        // Execute the query to get matching merge requests
        let mrs: Vec<MergeRequestInfo> = endpoint.query_async(&self.gl).await?;

        for mr in mrs {
            let endpoint = EditMergeRequest::builder()
                .project(&self.project_id)
                .merge_request(mr.iid)
                .state_event(MergeRequestStateEvent::Close)
                .build()?;

            ignore(endpoint).query_async(&self.gl).await?;
        }

        Ok(())
    }

    async fn delete_all_prs(&self) -> Result<()> {
        let endpoint =
            MergeRequests::builder().project(&self.project_id).build()?;

        // Execute the query to get matching merge requests
        let mrs: Vec<MergeRequestInfo> = endpoint.query_async(&self.gl).await?;

        for mr in mrs {
            let endpoint = DeleteMergeRequest::builder()
                .project(&self.project_id)
                .iid(mr.iid)
                .build()?;
            ignore(endpoint).query_async(&self.gl).await?;
        }

        Ok(())
    }

    async fn delete_all_releases(&self) -> Result<()> {
        let endpoint = ProjectReleases::builder()
            .project(&self.project_id)
            .build()?;

        let releases: Vec<GitlabRelease> =
            endpoint.query_async(&self.gl).await?;

        for release in releases {
            let endpoint = DeleteRelease::builder()
                .project(&self.project_id)
                .tag_name(release.tag_name)
                .build()?;

            ignore(endpoint).query_async(&self.gl).await?;
        }

        Ok(())
    }

    async fn delete_all_tags(&self) -> Result<()> {
        let endpoint = Tags::builder()
            .project(&self.project_id)
            .order_by(TagsOrderBy::Updated)
            .build()?;

        let tags: Vec<GitlabTag> = endpoint.query_async(&self.gl).await?;

        for tag in tags {
            let endpoint = DeleteTag::builder()
                .project(&self.project_id)
                .tag(tag.name)
                .build()?;
            ignore(endpoint).query_async(&self.gl).await?;
        }

        Ok(())
    }

    async fn delete_all_branches(&self) -> Result<()> {
        let endpoint = Branches::builder().project(&self.project_id).build()?;

        let branches: Vec<GitlabBranch> =
            endpoint.query_async(&self.gl).await?;

        for branch in branches {
            if branch.name == self.default_branch {
                continue;
            }

            let endpoint = DeleteBranch::builder()
                .project(&self.project_id)
                .branch(branch.name)
                .build()?;

            ignore(endpoint).query_async(&self.gl).await?;
        }

        Ok(())
    }

    async fn force_reset_history(&self) -> Result<()> {
        let default_renamed = format!("{}-old", self.default_branch);

        // rename main -> main-old
        let endpoint = CreateBranch::builder()
            .project(&self.project_id)
            .branch(&default_renamed)
            .ref_(&self.default_branch)
            .build()?;

        ignore(endpoint).query_async(&self.gl).await?;

        // set project default branch -> main-old
        let endpoint = EditProject::builder()
            .project(&self.project_id)
            .default_branch(&default_renamed)
            .build()?;

        ignore(endpoint).query_async(&self.gl).await?;

        // delete original default branch -> main
        let endpoint = DeleteBranch::builder()
            .project(&self.project_id)
            .branch(&self.default_branch)
            .build()?;

        ignore(endpoint).query_async(&self.gl).await?;

        // create new default branch using target commit -> main
        let endpoint = CreateBranch::builder()
            .project(&self.project_id)
            .branch(&self.default_branch)
            .ref_(&self.reset_sha)
            .build()?;

        ignore(endpoint).query_async(&self.gl).await?;

        // set project default branch back -> main
        let endpoint = EditProject::builder()
            .project(&self.project_id)
            .default_branch(&self.default_branch)
            .build()?;

        ignore(endpoint).query_async(&self.gl).await?;

        // delete temporary default branch -> main-old
        let endpoint = DeleteBranch::builder()
            .project(&self.project_id)
            .branch(&default_renamed)
            .build()?;

        ignore(endpoint).query_async(&self.gl).await?;

        Ok(())
    }
}

#[async_trait]
impl ForgeTestHelper for GitlabForgeTestHelper {
    async fn reset(&self) -> Result<()> {
        self.close_all_prs().await.unwrap();
        self.delete_all_prs().await.unwrap();
        self.delete_all_releases().await.unwrap();
        self.delete_all_tags().await.unwrap();
        self.delete_all_branches().await.unwrap();
        self.force_reset_history().await
    }

    async fn merge_pr(&self, pr_number: u64) -> Result<()> {
        let endpoint = MergeMergeRequest::builder()
            .project(&self.project_id)
            .merge_request(pr_number)
            .build()?;

        ignore(endpoint).query_async(&self.gl).await?;

        Ok(())
    }
}
