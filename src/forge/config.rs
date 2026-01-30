//! Configuration for Git forge platform connections.
use color_eyre::eyre::ContextCompat;
use derive_builder::Builder;
use git_url_parse::GitUrl;
use secrecy::{ExposeSecret, SecretString};
use std::env;

use crate::{ReleasaurusError, Result};

/// Default number of commits to search when finding releases.
pub const DEFAULT_COMMIT_SEARCH_DEPTH: u64 = 400;
/// Default number of tag to search when looking for starting tags
pub const DEFAULT_TAG_SEARCH_DEPTH: u8 = 100;
/// Default page size for paginated commit queries
pub const DEFAULT_PAGE_SIZE: u8 = 50;
/// Default branch name prefix for release PRs.
pub const DEFAULT_PR_BRANCH_PREFIX: &str = "releasaurus-release";
/// Default color for releasaurus labels in hex format.
pub const DEFAULT_LABEL_COLOR: &str = "a47dab";
/// Label applied to release PRs after tagging is complete.
pub const TAGGED_LABEL: &str = "releasaurus:tagged";
/// Label applied to release PRs while waiting for merge.
pub const PENDING_LABEL: &str = "releasaurus:pending";

/// Validate that repository URL uses HTTP or HTTPS scheme, rejecting SSH and
/// other protocols.
fn validate_scheme(scheme: git_url_parse::Scheme) -> Result<()> {
    match scheme {
        git_url_parse::Scheme::Http => Ok(()),
        git_url_parse::Scheme::Https => Ok(()),
        _ => Err(ReleasaurusError::InvalidRemoteUrl(
            "only http and https schemes are supported for repo urls"
                .to_string(),
        )),
    }
}

/// Resolve authentication token from multiple sources in priority order:
/// 1. Explicitly provided token (CLI argument)
/// 2. Token embedded in the URL
/// 3. Environment variable (forge-specific)
///
/// Returns an error if no token is found from any source.
pub fn resolve_token(
    cli_token: Option<SecretString>,
    url_token: Option<&String>,
    env_var: &str,
) -> Result<String> {
    cli_token
        .map(|t| t.expose_secret().to_string())
        .or_else(|| url_token.cloned())
        .or_else(|| env::var(env_var).ok())
        .ok_or_else(|| {
            ReleasaurusError::AuthenticationError(
                "Token not provided".to_string(),
            )
        })
}

/// Remote repository connection configuration for authenticating and
/// interacting with forge platforms.
#[derive(Debug, Clone, Builder)]
#[builder(setter(into, strip_option), default)]
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
        }
    }
}

impl RemoteConfig {
    pub fn from_url(url: GitUrl) -> Result<Self> {
        validate_scheme(url.scheme)?;

        let host = url.host.as_ref().ok_or_else(|| -> ReleasaurusError {
            ReleasaurusError::InvalidRemoteUrl(
                "unable to parse host from repo".to_string(),
            )
        })?;

        let owner = url.owner.as_ref().ok_or_else(|| -> ReleasaurusError {
            ReleasaurusError::InvalidRemoteUrl(
                "unable to parse owner from repo".to_string(),
            )
        })?;

        let project_path = url
            .path
            .strip_prefix("/")
            .wrap_err("failed to process project path")?
            .to_string();

        Ok(Self {
            host: host.clone(),
            port: url.port,
            scheme: url.scheme.to_string(),
            owner: owner.clone(),
            repo: url.name.clone(),
            path: project_path,
        })
    }

    pub fn link_base_url(&self) -> String {
        format!("{}://{}", self.scheme, self.host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_remote_config() {
        let remote = RemoteConfig::default();
        assert!(remote.port.is_none());
        assert_eq!(remote.host, "");
        assert_eq!(remote.scheme, "");
        assert_eq!(remote.owner, "");
        assert_eq!(remote.repo, "");
        assert_eq!(remote.path, "");
    }

    #[test]
    fn from_url_parses_https_github_url() {
        let url = GitUrl::parse("https://github.com/owner/repo").unwrap();
        let config = RemoteConfig::from_url(url).unwrap();

        assert_eq!(config.scheme, "https");
        assert_eq!(config.host, "github.com");
        assert_eq!(config.owner, "owner");
        assert_eq!(config.repo, "repo");
        assert_eq!(config.path, "owner/repo");
        assert!(config.port.is_none());
    }

    #[test]
    fn from_url_parses_http_url() {
        let url =
            GitUrl::parse("http://gitea.example.com/my-org/my-repo").unwrap();
        let config = RemoteConfig::from_url(url).unwrap();

        assert_eq!(config.scheme, "http");
        assert_eq!(config.host, "gitea.example.com");
        assert_eq!(config.owner, "my-org");
        assert_eq!(config.repo, "my-repo");
        assert_eq!(config.path, "my-org/my-repo");
    }

    #[test]
    fn from_url_parses_gitlab_nested_groups() {
        let url =
            GitUrl::parse("https://gitlab.com/group/subgroup/repo").unwrap();
        let config = RemoteConfig::from_url(url).unwrap();

        assert_eq!(config.scheme, "https");
        assert_eq!(config.host, "gitlab.com");
        // git_url_parse treats the immediate parent as owner
        assert_eq!(config.owner, "subgroup");
        assert_eq!(config.repo, "repo");
        // path contains the full project path for GitLab API calls
        assert_eq!(config.path, "group/subgroup/repo");
    }

    #[test]
    fn from_url_rejects_ssh_scheme() {
        let url = GitUrl::parse("git@github.com:owner/repo.git").unwrap();
        let result = RemoteConfig::from_url(url);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ReleasaurusError::InvalidRemoteUrl(_)));
    }

    #[test]
    fn from_url_rejects_git_scheme() {
        let url = GitUrl::parse("git://github.com/owner/repo.git").unwrap();
        let result = RemoteConfig::from_url(url);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ReleasaurusError::InvalidRemoteUrl(_)));
    }

    #[test]
    fn link_base_url_formats_https() {
        let config = RemoteConfig {
            scheme: "https".into(),
            host: "github.com".into(),
            ..Default::default()
        };

        assert_eq!(config.link_base_url(), "https://github.com");
    }

    #[test]
    fn link_base_url_formats_http() {
        let config = RemoteConfig {
            scheme: "http".into(),
            host: "gitlab.mycompany.com".into(),
            ..Default::default()
        };

        assert_eq!(config.link_base_url(), "http://gitlab.mycompany.com");
    }

    #[test]
    fn resolve_token_prefers_cli_token() {
        let cli_token = Some(SecretString::from("cli_token"));
        let url_token = Some(&"url_token".to_string());

        let result = resolve_token(cli_token, url_token, "NONEXISTENT_VAR");

        assert_eq!(result.unwrap(), "cli_token");
    }

    #[test]
    fn resolve_token_falls_back_to_url_token() {
        let url_token = Some(&"url_token".to_string());

        let result = resolve_token(None, url_token, "NONEXISTENT_VAR");

        assert_eq!(result.unwrap(), "url_token");
    }

    #[test]
    fn resolve_token_falls_back_to_env_var() {
        // Use a unique env var name to avoid conflicts
        let env_var = "RELEASAURUS_TEST_TOKEN_12345";
        // SAFETY: This test runs in isolation and uses a unique env var name
        unsafe {
            env::set_var(env_var, "env_token");
        }

        let result = resolve_token(None, None, env_var);

        // SAFETY: Cleaning up the env var we set above
        unsafe {
            env::remove_var(env_var);
        }

        assert_eq!(result.unwrap(), "env_token");
    }

    #[test]
    fn resolve_token_errors_when_no_token_available() {
        let result = resolve_token(None, None, "NONEXISTENT_VAR_67890");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ReleasaurusError::AuthenticationError(_)));
    }

    #[test]
    fn resolve_token_cli_takes_precedence_over_env() {
        let env_var = "RELEASAURUS_TEST_TOKEN_PRECEDENCE";
        // SAFETY: This test runs in isolation and uses a unique env var name
        unsafe {
            env::set_var(env_var, "env_token");
        }

        let cli_token = Some(SecretString::from("cli_token"));
        let result = resolve_token(cli_token, None, env_var);

        // SAFETY: Cleaning up the env var we set above
        unsafe {
            env::remove_var(env_var);
        }

        assert_eq!(result.unwrap(), "cli_token");
    }

    #[test]
    fn resolve_token_url_takes_precedence_over_env() {
        let env_var = "RELEASAURUS_TEST_TOKEN_URL_PRECEDENCE";
        // SAFETY: This test runs in isolation and uses a unique env var name
        unsafe {
            env::set_var(env_var, "env_token");
        }

        let url_token = Some(&"url_token".to_string());
        let result = resolve_token(None, url_token, env_var);

        // SAFETY: Cleaning up the env var we set above
        unsafe {
            env::remove_var(env_var);
        }

        assert_eq!(result.unwrap(), "url_token");
    }
}
