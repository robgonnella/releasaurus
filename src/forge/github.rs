//! Implements the Forge trait for Github
use color_eyre::eyre::{Report, eyre};
use log::*;
use octocrab::{
    Octocrab,
    params::{self, Direction, State},
};
use regex::Regex;
use tokio::runtime::Runtime;

use crate::{
    analyzer::release::Tag,
    forge::{
        config::{DEFAULT_LABEL_COLOR, PENDING_LABEL, RemoteConfig},
        request::{
            CreatePrRequest, ForgeCommit, GetPrRequest, PrLabelsRequest,
            ReleasePullRequest, UpdatePrRequest,
        },
        traits::Forge,
    },
    result::Result,
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

    fn get_latest_tag_for_prefix(&self, prefix: &str) -> Result<Option<Tag>> {
        let re = Regex::new(format!(r"^{prefix}").as_str())?;
        self.rt.block_on(async {
            if let Ok(octocrab) = self.new_instance() {
                let page = octocrab
                    .repos(&self.config.owner, &self.config.repo)
                    .list_tags()
                    .send()
                    .await?;

                for tag in page.into_iter() {
                    if re.is_match(&tag.name) {
                        let stripped =
                            re.replace_all(&tag.name, "").to_string();
                        if let Ok(sver) = semver::Version::parse(&stripped) {
                            return Ok(Some(Tag {
                                name: tag.name,
                                semver: sver,
                                sha: tag.commit.sha,
                            }));
                        }
                    }
                }
            }

            Ok(None)
        })
    }

    fn commit_iterator(
        &self,
        _since: Option<&str>,
        _max_depth: u64,
    ) -> Result<Vec<ForgeCommit>> {
        Err(eyre!("not implemented for gitea yet"))
    }

    fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<ReleasePullRequest>> {
        let prs = self.rt.block_on(async {
            let octocrab = self.new_instance()?;

            let handler = octocrab.pulls(&self.config.owner, &self.config.repo);

            handler
                .list()
                .state(params::State::Open)
                .head(req.head_branch)
                .send()
                .await
        })?;

        for pr in prs {
            if let Some(labels) = pr.labels
                && let Some(_pending_label) =
                    labels.iter().find(|l| l.name == PENDING_LABEL)
            {
                return Ok(Some(ReleasePullRequest {
                    number: pr.number,
                    sha: pr.head.sha,
                }));
            }
        }

        Ok(None)
    }

    fn get_merged_release_pr(&self) -> Result<Option<ReleasePullRequest>> {
        self.rt.block_on(async {
            let octocrab = self.new_instance().unwrap();

            let issues_handler =
                octocrab.issues(&self.config.owner, &self.config.repo);

            info!("looking for closed release prs with pending label");

            let issues = issues_handler
                .list()
                .direction(Direction::Descending)
                .labels(&[PENDING_LABEL.into()])
                .state(State::Closed)
                .per_page(2)
                .send()
                .await?;

            if issues.items.is_empty() {
              warn!(
                  r"No merged release PRs with the label {} found. Nothing to release",
                  PENDING_LABEL
              );
                return Ok(None);
            }

            if issues.items.len() > 1 {
                return Err(eyre!(
                  format!(
                    r"Found more than one closed release PR with pending label.
                    This mean either release PR were closed manually or releasaurus failed to remove tags.
                    You must remove the {} label from all closed release PRs except for the most recent.",
                    PENDING_LABEL
                )));
            }

            let issue = issues.items[0].clone();

            info!("found release pr: {}", issue.number);

            let pulls_handler = octocrab.pulls(&self.config.owner, &self.config.repo);

            let pr = pulls_handler.get(issue.number).await?;

            if let Some(merged) = pr.merged && !merged {
              return Err(eyre!(format!("found release PR {} but it hasn't been merged yet", pr.number)));
            }

            let sha = pr.merge_commit_sha.ok_or(eyre!("no merge_commit_sha found for pr"))?;

            Ok(Some(ReleasePullRequest{
              number: pr.number,
              sha,
            }))
        })
    }

    fn create_pr(&self, req: CreatePrRequest) -> Result<ReleasePullRequest> {
        self.rt.block_on(async {
            let octocrab = self.new_instance()?;

            let handler = octocrab.pulls(&self.config.owner, &self.config.repo);

            let pr = handler
                .create(req.title, req.head_branch, req.base_branch)
                .body(req.body)
                .send()
                .await?;

            Ok(ReleasePullRequest {
                number: pr.number,
                sha: pr.head.sha,
            })
        })
    }

    fn update_pr(&self, req: UpdatePrRequest) -> Result<()> {
        self.rt.block_on(async {
            let octocrab = self.new_instance()?;

            let pr_handler =
                octocrab.pulls(&self.config.owner, &self.config.repo);

            pr_handler
                .update(req.pr_number)
                .title(req.title)
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

            issue_handler
                .replace_all_labels(req.pr_number, &labels)
                .await?;

            Ok(())
        })
    }

    fn create_release(&self, tag: &str, sha: &str, notes: &str) -> Result<()> {
        self.rt.block_on(async {
            let octocrab = self.new_instance()?;

            octocrab
                .repos(&self.config.owner, &self.config.repo)
                .releases()
                .create(tag)
                .name(tag)
                .body(notes)
                .target_commitish(sha)
                .draft(false)
                .prerelease(false)
                .send()
                .await?;

            Ok::<(), Report>(())
        })
    }
}
