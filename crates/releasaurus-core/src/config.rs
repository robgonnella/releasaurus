use secrecy::Secret;

#[derive(Debug, Clone)]
/// Remote Repository configuration
pub struct RemoteConfig {
    /// The owner of the remote repo
    pub owner: String,
    /// The repo path i.e. <group>/<repo>
    pub repo: String,
    /// The access token for the remote repo
    pub token: Secret<String>,
    /// link_base_url for the remote
    /// This is only used for links displayed in changelog
    pub link_base_url: String,
    /// Optional api_url for the remote
    /// If you're updating base_url you should be updating this field as well
    pub api_url: Option<String>,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            owner: "".to_string(),
            repo: "".to_string(),
            token: Secret::from("".to_string()),
            link_base_url: "".to_string(),
            api_url: None,
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
