//! Configuration for Git forge platform connections.
use secrecy::SecretString;

pub const DEFAULT_PR_BRANCH_PREFIX: &str = "releasaurus-release";
pub const DEFAULT_LABEL_COLOR: &str = "a47dab";
pub const TAGGED_LABEL: &str = "releasaurus:tagged";
pub const PENDING_LABEL: &str = "releasaurus:pending";

use crate::{
    forge::{
        gitea::Gitea,
        github::Github,
        gitlab::Gitlab,
        traits::{FileLoader, Forge},
    },
    result::Result,
};

#[derive(Debug, Clone)]
/// Remote repository connection configuration.
pub struct RemoteConfig {
    /// Remote forge host
    pub host: String,
    /// Remote forge port
    pub port: Option<u16>,
    /// URL scheme (http or https).
    pub scheme: String,
    /// Repository owner.
    pub owner: String,
    /// Repository path.
    pub repo: String,
    /// Full repository path.
    pub path: String,
    /// Access token for authentication.
    pub token: SecretString,
    /// Base URL for commit links in changelog.
    pub commit_link_base_url: String,
    /// Base URL for release links in changelog.
    pub release_link_base_url: String,
    /// Max search depth for commits when starting sha not provided
    pub commit_search_depth: u64,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            host: "".to_string(),
            port: None,
            scheme: "".to_string(),
            owner: "".to_string(),
            repo: "".to_string(),
            path: "".to_string(),
            token: SecretString::from("".to_string()),
            commit_link_base_url: "".to_string(),
            release_link_base_url: "".to_string(),
            commit_search_depth: 0,
        }
    }
}

#[derive(Debug, Clone)]
/// Supported Git forge platforms.
pub enum Remote {
    Github(RemoteConfig),
    Gitlab(RemoteConfig),
    Gitea(RemoteConfig),
}

impl Remote {
    pub fn get_config(&self) -> RemoteConfig {
        match self {
            Remote::Github(conf) => conf.clone(),
            Remote::Gitlab(conf) => conf.clone(),
            Remote::Gitea(conf) => conf.clone(),
        }
    }

    pub async fn get_forge(&self) -> Result<Box<dyn Forge>> {
        match self {
            Remote::Github(config) => {
                let forge = Github::new(config.clone())?;
                Ok(Box::new(forge))
            }
            Remote::Gitlab(config) => {
                let forge = Gitlab::new(config.clone()).await?;
                Ok(Box::new(forge))
            }
            Remote::Gitea(config) => {
                let forge = Gitea::new(config.clone())?;
                Ok(Box::new(forge))
            }
        }
    }

    pub async fn get_file_loader(&self) -> Result<Box<dyn FileLoader>> {
        match self {
            Remote::Github(config) => {
                let forge = Github::new(config.clone())?;
                Ok(Box::new(forge))
            }
            Remote::Gitlab(config) => {
                let forge = Gitlab::new(config.clone()).await?;
                Ok(Box::new(forge))
            }
            Remote::Gitea(config) => {
                let forge = Gitea::new(config.clone())?;
                Ok(Box::new(forge))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_remote_config() {
        let remote = RemoteConfig::default();
        assert!(remote.port.is_none());
    }

    #[test]
    fn test_get_github_config() {
        let remote_config = RemoteConfig {
            host: "github.com".into(),
            ..RemoteConfig::default()
        };
        let remote = Remote::Github(remote_config.clone());
        let conf = remote.get_config();
        assert_eq!(conf.host, remote_config.host);
    }

    #[test]
    fn test_get_gitlab_config() {
        let remote_config = RemoteConfig {
            host: "gitlab.com".into(),
            ..RemoteConfig::default()
        };
        let remote = Remote::Gitlab(remote_config.clone());
        let conf = remote.get_config();
        assert_eq!(conf.host, remote_config.host);
    }

    #[test]
    fn test_get_gitea_config() {
        let remote_config = RemoteConfig {
            host: "gitea.com".into(),
            ..RemoteConfig::default()
        };
        let remote = Remote::Gitea(remote_config.clone());
        let conf = remote.get_config();
        assert_eq!(conf.host, remote_config.host);
    }
}
