pub const DEFAULT_PR_BRANCH_PREFIX: &str = "releasaurus-release--";
pub const PENDING_LABEL: &str = "releasaurus:pending";

use secrecy::Secret;

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
    /// The access token for the remote repo
    pub token: Secret<String>,
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
            token: Secret::from("".to_string()),
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
