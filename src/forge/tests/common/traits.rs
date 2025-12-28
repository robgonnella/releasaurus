use async_trait::async_trait;
use color_eyre::eyre::Result;

#[async_trait]
pub trait ForgeTestHelper {
    async fn reset(&self) -> Result<()>;
    async fn merge_pr(&self, pr_number: u64) -> Result<()>;
}
