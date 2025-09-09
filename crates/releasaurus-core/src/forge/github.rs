use color_eyre::eyre::Result;
use octocrab::{Octocrab, params};
use tokio::runtime::Runtime;

use crate::forge::{
    config::{DEFAULT_LABEL_COLOR, RemoteConfig},
    traits::Forge,
    types::{CreatePrRequest, GetPrRequest, PrLabelsRequest, UpdatePrRequest},
};

pub struct Github {
    config: RemoteConfig,
    base_uri: String,
    rt: Runtime,
}

impl Github {
    pub fn new(config: RemoteConfig) -> Result<Self> {
        let base_uri = format!("{}://api.{}", config.scheme, config.host);
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
        let builder = Octocrab::builder()
            .personal_token(self.config.token.clone())
            .base_uri(self.base_uri.clone())?;
        builder.build()
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
                .create(req.title, req.head_branch, req.base_branch)
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

    fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        self.rt.block_on(async {
            let octocrab = self.new_instance()?;

            let all_labels = octocrab
                .issues(&self.config.owner, &self.config.repo)
                .list_labels_for_repo()
                .per_page(100)
                .send()
                .await?;

            let mut labels = vec![];

            for name in req.labels {
                if let Some(label) =
                    all_labels.items.iter().find(|l| l.name == name)
                {
                    labels.push(label.name.clone())
                } else {
                    let label = octocrab
                        .issues(&self.config.owner, &self.config.repo)
                        .create_label(name, DEFAULT_LABEL_COLOR, "")
                        .await?;
                    labels.push(label.name);
                }
            }

            let issue_handler =
                octocrab.issues(&self.config.owner, &self.config.repo);

            issue_handler.add_labels(req.pr_number, &labels).await?;

            Ok(())
        })
    }
}
