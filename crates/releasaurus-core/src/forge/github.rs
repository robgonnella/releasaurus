use color_eyre::eyre::Result;
use octocrab::{Octocrab, models::pulls::PullRequest, params};
use secrecy::ExposeSecret;
use tokio::runtime::Runtime;

use crate::{
    config::RemoteConfig,
    forge::{
        traits::Forge,
        types::{
            CreatePrRequest, GetPrRequest, PrLabelsRequest, UpdatePrRequest,
        },
    },
};

pub struct Github {
    config: RemoteConfig,
    base_uri: String,
    rt: Runtime,
}

impl Github {
    pub fn new(config: RemoteConfig) -> Result<Self> {
        let base_uri = format!("{}://{}", config.scheme, config.host);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        Ok(Self {
            config,
            base_uri,
            rt,
        })
    }

    #[allow(clippy::result_large_err)]
    fn new_instance(&self) -> octocrab::Result<Octocrab> {
        let token = self.config.token.expose_secret().clone();
        let builder = Octocrab::builder()
            .personal_token(token)
            .base_uri(self.base_uri.clone())?;
        builder.build()
    }

    async fn get_pr_by_number(&self, pr_number: u64) -> Result<PullRequest> {
        let octocrab = octocrab::instance();
        let handler = octocrab.pulls(&self.config.owner, &self.config.repo);
        let pr = handler.get(pr_number).await?;
        Ok(pr)
    }
}

impl Forge for Github {
    fn get_pr_number(&self, req: GetPrRequest) -> Result<Option<u64>> {
        let prs = self.rt.block_on(async {
            let octocrab = self.new_instance()?;

            let handler = octocrab.pulls(&self.config.owner, &self.config.repo);

            handler
                .list()
                .state(params::State::Open)
                .head(req.head_branch)
                .base(req.base_branch)
                .send()
                .await
        })?;

        if let Some(pr) = prs.into_iter().last() {
            return Ok(Some(pr.number));
        }

        Ok(None)
    }

    fn create_pr(&self, req: CreatePrRequest) -> Result<u64> {
        self.rt.block_on(async {
            let octocrab = self.new_instance()?;

            let handler = octocrab.pulls(&self.config.owner, &self.config.repo);

            let pr = handler
                .create(req.title, req.base_branch, req.head_branch)
                .body(req.body)
                .send()
                .await?;

            Ok(pr.number)
        })
    }

    fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        self.rt.block_on(async {
            let octocrab = self.new_instance()?;

            let pr_handler =
                octocrab.pulls(&self.config.owner, &self.config.repo);

            pr_handler
                .update(req.pr_number)
                .body(req.body)
                .send()
                .await?;

            Ok(())
        })
    }

    fn add_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        self.rt.block_on(async {
            let octocrab = self.new_instance()?;
            let pr = self.get_pr_by_number(req.pr_number).await?;

            let mut labels = vec![];

            if let Some(pr_labels) = pr.labels {
                for label in pr_labels {
                    labels.push(label.name);
                }
            }

            labels.extend(req.labels);

            let issue_handler =
                octocrab.issues(&self.config.owner, &self.config.repo);

            issue_handler.add_labels(req.pr_number, &labels).await?;

            Ok(())
        })
    }

    fn remove_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        self.rt.block_on(async {
            let octocrab = self.new_instance()?;
            let pr = self.get_pr_by_number(req.pr_number).await?;

            if let Some(pr_labels) = pr.labels {
                let labels = pr_labels
                    .iter()
                    .filter(|l| !req.labels.contains(&l.name))
                    .map(|l| l.name.to_owned())
                    .collect::<Vec<String>>();

                let issue_handler = octocrab.issues(
                    self.config.owner.clone(),
                    self.config.repo.clone(),
                );

                issue_handler.add_labels(req.pr_number, &labels).await?;
            }

            Ok(())
        })
    }
}
