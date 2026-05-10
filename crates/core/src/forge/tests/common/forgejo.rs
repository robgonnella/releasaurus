use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::forge::{
    config::RepoUrl,
    tests::common::{gitea::GiteaForgeTestHelper, traits::ForgeTestHelper},
};

pub struct ForgejoForgeTestHelper {
    gitea_helper: GiteaForgeTestHelper,
}

impl ForgejoForgeTestHelper {
    pub async fn new(repo: &RepoUrl, token: &str, reset_sha: &str) -> Self {
        let gitea_helper =
            GiteaForgeTestHelper::new(repo, token, reset_sha).await;

        Self { gitea_helper }
    }
}

#[async_trait]
impl ForgeTestHelper for ForgejoForgeTestHelper {
    async fn reset(&self) -> Result<()> {
        self.gitea_helper.reset().await
    }

    async fn merge_pr(&self, pr_number: u64) -> Result<()> {
        self.gitea_helper.merge_pr(pr_number).await
    }
}
