use color_eyre::eyre::{Report, Result, WrapErr};
use gitea_sdk::{
    Client,
    model::{issues::Label, pulls::PullRequest},
};
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

pub struct Gitea {
    config: RemoteConfig,
    gt: Client,
    rt: Runtime,
}

impl Gitea {
    pub fn new(config: RemoteConfig) -> Result<Self> {
        let base_url = format!("{}://{}", config.scheme, config.host);
        let token = config.token.expose_secret();
        let gt = Client::new(base_url, gitea_sdk::Auth::Token(token));
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(Gitea { config, gt, rt })
    }

    async fn get_pr_by_number(&self, pr_number: u64) -> Result<PullRequest> {
        let handler = self
            .gt
            .pulls(self.config.owner.clone(), self.config.repo.clone());

        let pr = handler
            .get(pr_number as i64)
            .send(&self.gt)
            .await
            .wrap_err("failed to get pull request")?;

        Ok(pr)
    }
}

impl Forge for Gitea {
    fn get_pr_number(&self, req: GetPrRequest) -> Result<Option<u64>> {
        let pr_number = self.rt.block_on(async {
            let handler = self
                .gt
                .pulls(self.config.owner.clone(), self.config.repo.clone());

            let pr = handler
                .get_by_branches(req.head_branch, req.base_branch)
                .send(&self.gt)
                .await
                .wrap_err("failed to create pull request")?;

            let pr_num = pr.number.unsigned_abs();

            Ok::<u64, Report>(pr_num)
        })?;

        Ok(Some(pr_number))
    }

    fn create_pr(&self, req: CreatePrRequest) -> Result<u64> {
        let pr_number = self.rt.block_on(async {
            let handler = self
                .gt
                .pulls(self.config.owner.clone(), self.config.repo.clone());

            let pr = handler
                .create(req.target_branch, req.base_branch, req.title)
                .body(req.body)
                .send(&self.gt)
                .await
                .wrap_err("failed to create pull request")?;

            let pr_num = pr.number.unsigned_abs();

            Ok::<u64, Report>(pr_num)
        })?;

        Ok(pr_number)
    }

    fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        self.rt.block_on(async {
            let handler = self
                .gt
                .pulls(self.config.owner.clone(), self.config.repo.clone());

            handler
                .edit(req.pr_number as i64)
                .body(req.body)
                .send(&self.gt)
                .await
                .wrap_err("failed to update pull request")
        })?;

        Ok(())
    }

    fn add_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        self.rt.block_on(async {
            let pr = self.get_pr_by_number(req.pr_number).await?;

            let mut labels = vec![];

            for label in pr.labels {
                labels.push(label)
            }

            for label in req.labels {
                let new_label = Label {
                    name: label,
                    ..Label::default()
                };
                labels.push(new_label);
            }

            let label_ids: Vec<i64> = labels.iter().map(|l| l.id).collect();

            self.gt
                .pulls(&self.config.owner, &self.config.repo)
                .edit(pr.number)
                .labels(label_ids)
                .send(&self.gt)
                .await?;

            Ok::<(), Report>(())
        })?;

        Ok(())
    }

    fn remove_pr_labels(&self, req: PrLabelsRequest) -> Result<()> {
        self.rt.block_on(async {
            let pr = self.get_pr_by_number(req.pr_number).await?;

            let labels: Vec<i64> = pr
                .labels
                .iter()
                .filter(|l| !req.labels.contains(&l.name))
                .map(|l| l.id)
                .collect();

            self.gt
                .pulls(&self.config.owner, &self.config.repo)
                .edit(pr.number)
                .labels(labels)
                .send(&self.gt)
                .await?;

            Ok::<(), Report>(())
        })?;

        Ok(())
    }
}
