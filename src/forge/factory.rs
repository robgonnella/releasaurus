//! Factory for creating forge implementations based on configuration.

use crate::{
    Result,
    forge::{
        config::{Remote, RemoteConfig},
        gitea::Gitea,
        github::Github,
        gitlab::Gitlab,
        local::LocalRepo,
        manager::ForgeManager,
        traits::Forge,
    },
};

/// Factory for creating forge implementations.
pub struct ForgeFactory;

impl ForgeFactory {
    /// Create a ForgeManager instance based on the Remote configuration.
    pub async fn create(remote: &Remote) -> Result<ForgeManager> {
        let forge: Box<dyn Forge> = match remote {
            Remote::Github(config) => Self::create_github(config).await?,
            Remote::Gitlab(config) => Self::create_gitlab(config).await?,
            Remote::Gitea(config) => Self::create_gitea(config).await?,
            Remote::Local(repo_path) => Self::create_local(repo_path)?,
        };

        Ok(ForgeManager::new(forge))
    }

    async fn create_github(config: &RemoteConfig) -> Result<Box<dyn Forge>> {
        Ok(Box::new(Github::new(config.clone()).await?))
    }

    async fn create_gitlab(config: &RemoteConfig) -> Result<Box<dyn Forge>> {
        Ok(Box::new(Gitlab::new(config.clone()).await?))
    }

    async fn create_gitea(config: &RemoteConfig) -> Result<Box<dyn Forge>> {
        Ok(Box::new(Gitea::new(config.clone()).await?))
    }

    fn create_local(repo_path: &str) -> Result<Box<dyn Forge>> {
        Ok(Box::new(LocalRepo::new(repo_path.to_string())?))
    }
}
