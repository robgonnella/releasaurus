//! Configuration for Git forge platform connections.
use secrecy::SecretString;

pub const DEFAULT_PR_BRANCH_PREFIX: &str = "releasaurus-release--";
pub const DEFAULT_LABEL_COLOR: &str = "a47dab";
pub const TAGGED_LABEL: &str = "releasaurus:tagged";
pub const PENDING_LABEL: &str = "releasaurus:pending";

use crate::{
    forge::{gitea::Gitea, github::Github, gitlab::Gitlab, traits::Forge},
    result::Result,
};

#[derive(Debug, Clone)]
/// Remote repository connection configuration.
pub struct RemoteConfig {
    /// Remote repository host.
    pub host: String,
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
    pub fn get_forge(&self) -> Result<Box<dyn Forge>> {
        match self {
            Remote::Github(config) => {
                let forge = Github::new(config.clone())?;
                Ok(Box::new(forge))
            }
            Remote::Gitlab(config) => {
                let forge = Gitlab::new(config.clone())?;
                Ok(Box::new(forge))
            }
            Remote::Gitea(config) => {
                let forge = Gitea::new(config.clone())?;
                Ok(Box::new(forge))
            }
        }
    }
}
