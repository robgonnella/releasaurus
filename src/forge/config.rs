//! Configuration used implement various forges
use color_eyre::eyre::Result;
use secrecy::SecretString;

pub const DEFAULT_PR_BRANCH_PREFIX: &str = "releasaurus-release--";
pub const DEFAULT_LABEL_COLOR: &str = "a47dab";
pub const TAGGED_LABEL: &str = "releasaurus:tagged";
pub const PENDING_LABEL: &str = "releasaurus:pending";

use crate::forge::{
    gitea::Gitea, github::Github, gitlab::Gitlab, traits::Forge,
};

#[derive(Debug, Clone)]
/// Remote Repository configuration
pub struct RemoteConfig {
    /// The host for this remote repo
    pub host: String,
    /// The scheme for this remote repo http|https
    pub scheme: String,
    /// The owner of the remote repo
    pub owner: String,
    /// The repo path i.e. <group>/<repo>
    pub repo: String,
    /// The full path to the repo i.e. /org/group/owner/repo
    pub path: String,
    /// The access token for the remote repo
    pub token: SecretString,
    /// commit link base_url for the remote
    /// This is only used for links displayed in changelog
    pub commit_link_base_url: String,
    /// release link base_url for the remote
    /// This is only used for links displayed in changelog
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
/// Represents the valid types of remotes
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
