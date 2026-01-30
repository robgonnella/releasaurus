use async_trait::async_trait;
use color_eyre::eyre::Result;
use git_url_parse::GitUrl;
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use serde::Deserialize;
use url::Url;

use crate::forge::tests::common::traits::ForgeTestHelper;

#[derive(Debug, Deserialize)]
struct GiteaIssue {
    number: u64,
}

#[derive(Debug, Deserialize)]
struct GiteaRelease {
    pub id: u64,
}

#[derive(Debug, Deserialize)]
struct GiteaTag {
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct GiteaBranch {
    pub name: String,
}

pub struct GiteaForgeTestHelper {
    client: Client,
    base_url: Url,
    default_branch: String,
    reset_sha: String,
}

impl GiteaForgeTestHelper {
    pub async fn new(repo: &GitUrl, token: &str, reset_sha: &str) -> Self {
        let mut headers = HeaderMap::new();

        let token_value =
            HeaderValue::from_str(format!("token {}", token).as_str()).unwrap();

        headers.append("Authorization", token_value);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        let host = repo.host.as_ref().unwrap().clone();

        let mut base_url = format!(
            "{}://{}/api/v1/repos/{}/",
            repo.scheme, host, repo.fullname
        );

        if let Some(port) = repo.port {
            base_url = format!(
                "{}://{}:{}/api/v1/repos/{}/",
                repo.scheme, host, port, repo.fullname
            );
        }

        let base_url = Url::parse(&base_url).unwrap();

        let request = client.get(base_url.clone()).build().unwrap();
        let response = client.execute(request).await.unwrap();
        let result = response.error_for_status().unwrap();
        let repo: serde_json::Value = result.json().await.unwrap();

        let default_branch =
            repo["default_branch"].as_str().unwrap().to_string();

        Self {
            client,
            base_url,
            default_branch,
            reset_sha: reset_sha.into(),
        }
    }

    async fn close_all_prs(&self) -> Result<()> {
        let issues_url = self
            .base_url
            .join("issues?state=open&type=pulls&limit=9999")?;

        let request = self.client.get(issues_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let issues: Vec<GiteaIssue> = result.json().await?;

        if issues.is_empty() {
            return Ok(());
        }

        for issue in issues.iter() {
            let body = serde_json::json!({
              "state": "closed"
            });

            let pr_url =
                self.base_url.join(&format!("issues/{}", issue.number))?;

            let request = self.client.patch(pr_url).json(&body).build()?;

            let response = self.client.execute(request).await?;
            response.error_for_status()?;
        }

        Ok(())
    }

    async fn delete_all_prs(&self) -> Result<()> {
        let issues_url = self
            .base_url
            .join("issues?state=closed&type=pulls&limit=9999")?;

        let request = self.client.get(issues_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let issues: Vec<GiteaIssue> = result.json().await?;

        if issues.is_empty() {
            return Ok(());
        }

        for issue in issues.iter() {
            let pr_url =
                self.base_url.join(&format!("issues/{}", issue.number))?;
            let request = self.client.delete(pr_url).build()?;
            let response = self.client.execute(request).await?;
            response.error_for_status()?;
        }

        Ok(())
    }

    async fn delete_all_releases(&self) -> Result<()> {
        let releases_url = self.base_url.join("releases?limit=9999")?;
        let request = self.client.get(releases_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let releases: Vec<GiteaRelease> = result.json().await?;

        if releases.is_empty() {
            return Ok(());
        }

        for release in releases.iter() {
            let release_url =
                self.base_url.join(&format!("releases/{}", release.id))?;
            let request = self.client.delete(release_url).build()?;
            let response = self.client.execute(request).await?;
            response.error_for_status()?;
        }

        Ok(())
    }

    async fn delete_all_tags(&self) -> Result<()> {
        let tags_url = self.base_url.join("tags?limit=9999")?;
        let request = self.client.get(tags_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let tags: Vec<GiteaTag> = result.json().await?;

        if tags.is_empty() {
            return Ok(());
        }

        for tag in tags.iter() {
            let tag_url = self.base_url.join(&format!("tags/{}", tag.name))?;
            let request = self.client.delete(tag_url).build()?;
            let response = self.client.execute(request).await?;
            response.error_for_status()?;
        }

        Ok(())
    }

    async fn delete_all_branches(&self) -> Result<()> {
        let branches_url = self.base_url.join("branches?limit=9999")?;
        let request = self.client.get(branches_url).build()?;
        let response = self.client.execute(request).await?;
        let result = response.error_for_status()?;
        let branches: Vec<GiteaBranch> = result.json().await?;

        if branches.is_empty() {
            return Ok(());
        }

        for branch in branches.iter() {
            if branch.name == self.default_branch {
                continue;
            }

            let branch_url =
                self.base_url.join(&format!("branches/{}", branch.name))?;
            let request = self.client.delete(branch_url).build()?;
            let response = self.client.execute(request).await?;
            response.error_for_status()?;
        }

        Ok(())
    }

    async fn force_reset_history(&self) -> Result<()> {
        let default_renamed = format!("{}-old", self.default_branch);

        // rename original default branch -> main -> main-old
        let branch_url = self
            .base_url
            .join(&format!("branches/{}", self.default_branch))?;

        let body = serde_json::json!({
          "name": &default_renamed,
        });

        let request = self.client.patch(branch_url).json(&body).build()?;
        let response = self.client.execute(request).await?;
        response.error_for_status()?;

        // create new default branch using reset_sha -> main
        let branch_url = self.base_url.join("branches")?;

        let body = serde_json::json!({
          "new_branch_name": &self.default_branch,
          "old_ref_name": &self.reset_sha
        });

        let request = self.client.post(branch_url).json(&body).build()?;
        let response = self.client.execute(request).await?;
        response.error_for_status()?;

        // set new branch as default -> main
        let repo_url = self.base_url.clone();

        let body = serde_json::json!({
          "default_branch": self.default_branch,
        });

        let request = self.client.patch(repo_url).json(&body).build()?;
        let response = self.client.execute(request).await?;
        response.error_for_status()?;

        // delete old default branch -> main-old
        let branch_url = self
            .base_url
            .join(&format!("branches/{}", default_renamed))?;
        let request = self.client.delete(branch_url).build()?;
        let response = self.client.execute(request).await?;
        response.error_for_status()?;

        Ok(())
    }
}

#[async_trait]
impl ForgeTestHelper for GiteaForgeTestHelper {
    async fn reset(&self) -> Result<()> {
        self.close_all_prs().await.unwrap();
        self.delete_all_prs().await.unwrap();
        self.delete_all_releases().await.unwrap();
        self.delete_all_tags().await.unwrap();
        self.delete_all_branches().await.unwrap();
        self.force_reset_history().await
    }

    async fn merge_pr(&self, pr_number: u64) -> Result<()> {
        let endpoint =
            self.base_url.join(&format!("pulls/{}/merge", pr_number))?;
        let body = serde_json::json!({
          "Do": "merge"
        });
        let request = self.client.post(endpoint).json(&body).build()?;
        let response = self.client.execute(request).await?;
        response.error_for_status()?;
        Ok(())
    }
}
