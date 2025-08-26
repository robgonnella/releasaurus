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
    types::{CreatePrRequest, GetPrRequest, PrLabelsRequest, UpdatePrRequest},
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
struct PullRequest {
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

    fn get_pr_number(&self, req: GetPrRequest) -> Result<Option<u64>> {
        let pulls_url = self.base_url.join(
            format!("pulls/{}/{}", req.base_branch, req.head_branch).as_str(),
        )?;
        let request = self.client.get(pulls_url).build()?;
        let response = self.client.execute(request)?;
        let result = response.error_for_status()?;
        let pr: PullRequest = result.json()?;
        Ok(Some(pr.number))
    }

    fn create_pr(&self, req: CreatePrRequest) -> Result<u64> {
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
        Ok(pr.number)
    }

    fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        let data = UpdatePullBody { body: req.body };
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
