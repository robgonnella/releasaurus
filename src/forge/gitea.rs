//! Implements the Forge trait for Gitea
use color_eyre::eyre::Result;
use reqwest::{
    Url,
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

use crate::forge::{
    config::{DEFAULT_LABEL_COLOR, RemoteConfig},
    traits::Forge,
    types::{
        CreatePrRequest, GetPrRequest, PrLabelsRequest, ReleasePullRequest,
        UpdatePrRequest,
    },
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
    title: String,
    body: String,
    labels: Vec<Label>,
    head: PullRequestHead,
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
                "{}://{}/api/v1/repos/{}/{}/",
                config.scheme, config.host, config.owner, config.repo
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

    fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<ReleasePullRequest>> {
        let pulls_url = self.base_url.join(
            format!("pulls/{}/{}?state=open", req.base_branch, req.head_branch)
                .as_str(),
        )?;
        let request = self.client.get(pulls_url).build()?;
        let response = self.client.execute(request)?;
        let result = response.error_for_status()?;
        let pr: PullRequest = result.json()?;

        let labels = pr
            .labels
            .iter()
            .map(|l| l.name.clone())
            .collect::<Vec<String>>();

        Ok(Some(ReleasePullRequest {
            number: pr.number,
            sha: pr.head.sha,
            title: pr.title,
            body: pr.body.clone(),
            labels,
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
            title: pr.title,
            body: pr.body.clone(),
            labels: pr
                .labels
                .iter()
                .map(|l| l.name.clone())
                .collect::<Vec<String>>(),
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
}

#[cfg(test)]
#[cfg(feature = "forge-tests")]
mod tests {
    use std::collections::HashMap;
    use std::env;

    use secrecy::SecretString;

    use crate::forge::config::PENDING_LABEL;

    use super::*;

    fn delete_label(
        client: &Client,
        base_url: &Url,
        label_name: String,
    ) -> Result<()> {
        let labels_url = base_url.join("labels")?;
        let request = client.get(labels_url).build()?;
        let response = client.execute(request)?;
        let result = response.error_for_status()?;
        let labels: Vec<Label> = result.json()?;

        for label in labels {
            if label.name == label_name {
                let label_path = format!("labels/{}", label.id);
                let delete_url = base_url.join(label_path.as_str())?;
                let request = client.delete(delete_url).build()?;
                let response = client.execute(request)?;
                response.error_for_status()?;
                return Ok(());
            }
        }

        Ok(())
    }

    fn close_pr(client: &Client, base_url: &Url, pr_number: u64) -> Result<()> {
        let mut body = HashMap::new();
        body.insert("state", "closed");
        let pulls_url =
            base_url.join(format!("pulls/{}", pr_number).as_str())?;
        let request = client.patch(pulls_url).json(&body).build()?;
        let response = client.execute(request)?;
        response.error_for_status()?;
        Ok(())
    }

    #[test]
    fn test_gitea_forge() {
        let result = env::var("GT_TEST_TOKEN");
        assert!(
            result.is_ok(),
            "must set GT_TEST_TOKEN as environment variable to run these tests"
        );

        let token = result.unwrap();

        let remote_config = RemoteConfig {
            scheme: "https".into(),
            host: "gitea.com".into(),
            owner: "rgon".into(),
            repo: "test-repo".into(),
            path: format!("{}/{}", "rgon", "test-repo"),
            token: SecretString::from(token.clone()),
            commit_link_base_url: "".into(),
            release_link_base_url: "".into(),
        };

        let mut headers = HeaderMap::new();

        let token_value =
            HeaderValue::from_str(format!("token {}", token).as_str()).unwrap();

        headers.append("Authorization", token_value);

        let client = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        let base_url = Url::parse(
            format!(
                "{}://{}/api/v1/repos/{}/{}/",
                remote_config.scheme,
                remote_config.host,
                remote_config.owner,
                remote_config.repo
            )
            .as_str(),
        )
        .unwrap();

        let result = Gitea::new(remote_config.clone());
        assert!(result.is_ok(), "failed to create gitea forge");
        let forge = result.unwrap();

        let req = CreatePrRequest {
            head_branch: "test-branch".into(),
            base_branch: "main".into(),
            body: "super duper!".into(),
            title: "The is my test PR".into(),
        };

        let result = forge.create_pr(req);
        assert!(result.is_ok(), "failed to create PR");
        let pr = result.unwrap();
        let pr_number = pr.number;

        let req = UpdatePrRequest {
            pr_number,
            title: "This is my updated title".into(),
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
        let result = forge.get_open_release_pr(req);
        assert!(result.is_ok(), "failed to get PR number");

        let result = close_pr(&client, &base_url, pr_number);
        assert!(result.is_ok(), "failed to close PR");

        let result = delete_label(&client, &base_url, new_label);
        assert!(result.is_ok(), "failed to delete label")
    }
}
