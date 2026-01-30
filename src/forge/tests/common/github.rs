use async_trait::async_trait;
use color_eyre::eyre::Result;
use git_url_parse::GitUrl;
use octocrab::{
    Octocrab,
    params::{self, pulls, repos::Reference},
};

use crate::forge::tests::common::traits::ForgeTestHelper;

pub struct GithubForgeTestHelper {
    instance: Octocrab,
    base_uri: String,
    owner: String,
    repo: String,
    default_branch: String,
    reset_sha: String,
}

impl GithubForgeTestHelper {
    pub async fn new(repo: &GitUrl, token: &str, reset_sha: &str) -> Self {
        let host = repo.host.as_ref().unwrap().clone();
        let owner = repo.owner.as_ref().unwrap().clone();
        let repo_str = repo.name.clone();

        let base_uri = format!("{}://api.{}", repo.scheme, host);

        let builder = Octocrab::builder()
            .personal_token(token)
            .base_uri(base_uri.clone())
            .unwrap();
        let instance = builder.build().unwrap();

        let repo = instance.repos(&owner, &repo_str).get().await.unwrap();
        let default_branch = repo.default_branch.unwrap();

        Self {
            base_uri,
            owner,
            repo: repo_str,
            instance,
            default_branch,
            reset_sha: reset_sha.into(),
        }
    }

    async fn close_all_prs(&self) -> Result<()> {
        let pulls = self
            .instance
            .pulls(&self.owner, &self.repo)
            .list()
            .state(params::State::All)
            .send()
            .await?;

        for pull in &pulls {
            self.instance
                .issues(&self.owner, &self.repo)
                .replace_all_labels(pull.number, &[])
                .await?;

            self.instance
                .pulls(&self.owner, &self.repo)
                .update(pull.number)
                .state(pulls::State::Closed)
                .send()
                .await?;
        }

        Ok(())
    }

    async fn delete_all_releases(&self) -> Result<()> {
        let releases = self
            .instance
            .repos(&self.owner, &self.repo)
            .releases()
            .list()
            .send()
            .await?;

        for release in releases {
            self.instance
                .repos(&self.owner, &self.repo)
                .releases()
                .delete(release.id.0)
                .await?;
        }

        Ok(())
    }

    async fn delete_all_tags(&self) -> Result<()> {
        let tags = self
            .instance
            .repos(&self.owner, &self.repo)
            .list_tags()
            .send()
            .await?;

        for tag in tags {
            self.instance
                .repos(&self.owner, &self.repo)
                .delete_ref(&Reference::Tag(tag.name))
                .await?;
        }

        Ok(())
    }

    async fn delete_all_branches(&self) -> Result<()> {
        let branches = self
            .instance
            .repos(&self.owner, &self.repo)
            .list_branches()
            .send()
            .await?;

        for branch in branches {
            if branch.name == self.default_branch {
                continue;
            }

            self.instance
                .repos(&self.owner, &self.repo)
                .delete_ref(&Reference::Branch(branch.name))
                .await?;
        }

        Ok(())
    }

    async fn force_reset_history(&self) -> Result<()> {
        let route = format!(
            "{}/repos/{}/{}/git/refs/heads/{}",
            self.base_uri, self.owner, self.repo, self.default_branch
        );

        let body = serde_json::json!({
            "sha": &self.reset_sha,
            "force": true
        });

        let _: serde_json::Value =
            self.instance.patch(route, Some(&body)).await?;

        Ok(())
    }
}

#[async_trait]
impl ForgeTestHelper for GithubForgeTestHelper {
    async fn reset(&self) -> Result<()> {
        self.close_all_prs().await.unwrap();
        self.delete_all_releases().await.unwrap();
        self.delete_all_tags().await.unwrap();
        self.delete_all_branches().await.unwrap();
        self.force_reset_history().await
    }

    async fn merge_pr(&self, pr_number: u64) -> Result<()> {
        self.instance
            .pulls(&self.owner, &self.repo)
            .merge(pr_number)
            .send()
            .await?;

        Ok(())
    }
}
