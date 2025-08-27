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
    fn config(&self) -> &RemoteConfig {
        &self.config
    }

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

#[cfg(test)]
#[cfg(feature = "forge-tests")]
mod tests {
    use std::env;

    use color_eyre::eyre::Result;
    use octocrab::{Octocrab, params};
    use secrecy::SecretString;

    use crate::forge::{
        config::{PENDING_LABEL, RemoteConfig},
        github::Github,
        traits::Forge,
        types::{
            CreatePrRequest, GetPrRequest, PrLabelsRequest, UpdatePrRequest,
        },
    };

    fn delete_label(config: &RemoteConfig, label: String) -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        rt.block_on(async {
            let builder = Octocrab::builder()
                .personal_token(config.token.clone())
                .base_uri(format!("{}://api.{}", config.scheme, config.host))?;

            let octocrab = builder.build()?;

            octocrab
                .issues(&config.owner, &config.repo)
                .delete_label(label)
                .await?;

            Ok(())
        })
    }

    fn close_pr(config: &RemoteConfig, pr_number: u64) -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        rt.block_on(async {
            let builder = Octocrab::builder()
                .personal_token(config.token.clone())
                .base_uri(format!("{}://api.{}", config.scheme, config.host))?;

            let octocrab = builder.build()?;

            octocrab
                .pulls(&config.owner, &config.repo)
                .update(pr_number)
                .state(params::pulls::State::Closed)
                .send()
                .await?;

            Ok(())
        })
    }

    #[test]
    fn test_github_forge() {
        let result = env::var("GH_TEST_TOKEN");
        assert!(
            result.is_ok(),
            "must set GH_TEST_TOKEN as environment variable to run these tests"
        );

        let token = result.unwrap();

        let remote_config = RemoteConfig {
            scheme: "https".into(),
            host: "github.com".into(),
            owner: "robgonnella".into(),
            repo: "test-repo".into(),
            token: SecretString::from(token),
            commit_link_base_url: "".into(),
            release_link_base_url: "".into(),
        };

        let result = Github::new(remote_config.clone());
        assert!(result.is_ok(), "failed to create github forge");
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

        let result = close_pr(&remote_config, pr_number);
        assert!(result.is_ok(), "failed to close PR");

        let result = delete_label(&remote_config, new_label);
        assert!(result.is_ok(), "failed to delete label")
    }
}
