//! Configuration for Git forge platform connections.
use secrecy::SecretString;

/// Default number of commits to search when finding releases.
pub const DEFAULT_COMMIT_SEARCH_DEPTH: u64 = 400;
/// Default page size for paginated commit queries
pub const DEFAULT_PAGE_SIZE: u8 = 100;
/// Default branch name prefix for release PRs.
pub const DEFAULT_PR_BRANCH_PREFIX: &str = "releasaurus-release";
/// Default color for releasaurus labels in hex format.
pub const DEFAULT_LABEL_COLOR: &str = "a47dab";
/// Label applied to release PRs after tagging is complete.
pub const TAGGED_LABEL: &str = "releasaurus:tagged";
/// Label applied to release PRs while waiting for merge.
pub const PENDING_LABEL: &str = "releasaurus:pending";

use crate::{
    forge::{gitea::Gitea, github::Github, gitlab::Gitlab, traits::Forge},
    result::Result,
};

/// Remote repository connection configuration for authenticating and
/// interacting with forge platforms.
#[derive(Debug, Clone)]
pub struct RemoteConfig {
    /// Remote forge host (e.g., "github.com").
    pub host: String,
    /// Remote forge port for self-hosted instances.
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
    /// Create forge client instance for the configured platform.
    pub async fn get_forge(&self) -> Result<Box<dyn Forge>> {
        match self {
            Remote::Github(config) => {
                let forge = Github::new(config.clone()).await?;
                Ok(Box::new(forge))
            }
            Remote::Gitlab(config) => {
                let forge = Gitlab::new(config.clone()).await?;
                Ok(Box::new(forge))
            }
            Remote::Gitea(config) => {
                let forge = Gitea::new(config.clone()).await?;
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
}
