//! Configuration and URL types for Git forge platform connections.
use secrecy::SecretString;
use std::env;

use crate::result::{ReleasaurusError, Result};

/// URL scheme for a remote repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scheme {
    Http,
    Https,
}

impl std::fmt::Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scheme::Http => write!(f, "http"),
            Scheme::Https => write!(f, "https"),
        }
    }
}

/// A repository URL with its parsed components.
///
/// Construct this directly from the components of your parsed URL.
/// The CLI crate uses `git_url_parse` to populate it; library
/// consumers can use any URL parser they prefer.
///
/// `Display` produces `scheme://host[:port]/path`.
#[derive(Debug, Clone)]
pub struct RepoUrl {
    /// Remote forge host (e.g. `"github.com"`).
    pub host: String,
    /// Repository owner or organisation.
    pub owner: String,
    /// Repository name.
    pub name: String,
    /// Full project path (e.g. `"owner/repo"` or
    /// `"group/subgroup/repo"` for nested GitLab groups).
    pub path: String,
    /// Optional port for self-hosted instances.
    pub port: Option<u16>,
    /// URL scheme.
    pub scheme: Scheme,
    /// Token embedded in the URL (e.g.
    /// `https://TOKEN@github.com/...`). Set to `None` if the URL
    /// contains no embedded token. [`SecretString`] comes from the
    /// [`secrecy`](https://docs.rs/secrecy) crate.
    pub token: Option<SecretString>,
}

impl RepoUrl {
    pub fn link_base_url(&self) -> String {
        match self.port {
            Some(port) => format!("{}://{}:{}", self.scheme, self.host, port),
            None => format!("{}://{}", self.scheme, self.host),
        }
    }
}

impl std::fmt::Display for RepoUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.port {
            Some(port) => write!(
                f,
                "{}://{}:{}/{}",
                self.scheme, self.host, port, self.path
            ),
            None => write!(f, "{}://{}/{}", self.scheme, self.host, self.path),
        }
    }
}

/// Default page size for paginated commit queries
pub const DEFAULT_PAGE_SIZE: u8 = 50;
/// Default branch name prefix for release PRs.
pub const DEFAULT_PR_BRANCH_PREFIX: &str = "releasaurus-release";
/// Default color for releasaurus labels in hex format.
pub const DEFAULT_LABEL_COLOR: &str = "a47dab";
/// Label applied to release PRs after tagging is complete.
/// Uses the GitLab scoped-label separator `::` so that applying
/// this label automatically removes `PENDING_LABEL` on GitLab.
/// On GitHub and Gitea the `::` is treated as literal characters.
pub const TAGGED_LABEL: &str = "releasaurus::tagged";
/// Label applied to release PRs while waiting for merge.
/// Uses the GitLab scoped-label separator `::` so that applying
/// this label automatically removes `TAGGED_LABEL` on GitLab.
/// On GitHub and Gitea the `::` is treated as literal characters.
pub const PENDING_LABEL: &str = "releasaurus::pending";
/// Legacy pending label used before the scoped-label migration.
/// Retained so that existing release PRs created by an older
/// version of releasaurus can still be found after an upgrade.
pub const LEGACY_PENDING_LABEL: &str = "releasaurus:pending";

/// Represents the default token variable names that are checked for
/// authenticating each forge type
#[derive(Clone, Copy, strum::Display)]
pub enum TokenVar {
    #[strum(to_string = "GITHUB_TOKEN")]
    Github,
    #[strum(to_string = "GITLAB_TOKEN")]
    Gitlab,
    #[strum(to_string = "GITEA_TOKEN")]
    Gitea,
}

/// Resolve authentication token from multiple sources in priority order:
/// 1. Explicitly provided token (CLI argument)
/// 2. Token embedded in the URL
/// 3. Environment variable (forge-specific)
///
/// Returns an error if no token is found from any source.
pub fn resolve_token(
    cli_token: Option<SecretString>,
    url_token: Option<&SecretString>,
    token_var: TokenVar,
) -> Result<SecretString> {
    cli_token
        .or_else(|| url_token.cloned())
        .or_else(|| {
            env::var(token_var.to_string()).ok().map(SecretString::from)
        })
        .ok_or_else(|| {
            ReleasaurusError::AuthenticationError(
                "Token not provided".to_string(),
            )
        })
}

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use super::*;

    #[test]
    fn resolve_token_prefers_cli_token() {
        let cli_token = Some(SecretString::from("cli_token"));
        let url_token = SecretString::from("url_token");

        let result =
            resolve_token(cli_token, Some(&url_token), TokenVar::Github);

        assert_eq!(result.unwrap().expose_secret(), "cli_token");
    }

    #[test]
    fn resolve_token_falls_back_to_url_token() {
        let url_token = SecretString::from("url_token");

        let result = resolve_token(None, Some(&url_token), TokenVar::Gitlab);

        assert_eq!(result.unwrap().expose_secret(), "url_token");
    }

    #[test]
    fn resolve_token_falls_back_to_env_var() {
        temp_env::with_var(
            TokenVar::Github.to_string(),
            Some("env_token"),
            || {
                let result = resolve_token(None, None, TokenVar::Github);
                assert_eq!(result.unwrap().expose_secret(), "env_token");
            },
        );
    }

    #[test]
    fn resolve_token_errors_when_no_token_available() {
        temp_env::with_var_unset(TokenVar::Gitea.to_string(), || {
            let result = resolve_token(None, None, TokenVar::Gitea);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, ReleasaurusError::AuthenticationError(_)));
        });
    }

    #[test]
    fn link_base_url_without_port() {
        let url = RepoUrl {
            scheme: Scheme::Https,
            host: "gitea.example.com".to_string(),
            owner: "org".to_string(),
            name: "repo".to_string(),
            path: "/org/repo".to_string(),
            port: None,
            token: None,
        };
        assert_eq!(url.link_base_url(), "https://gitea.example.com");
    }

    #[test]
    fn link_base_url_with_port() {
        let url = RepoUrl {
            scheme: Scheme::Https,
            host: "gitea.example.com".to_string(),
            owner: "org".to_string(),
            name: "repo".to_string(),
            path: "/org/repo".to_string(),
            port: Some(3000),
            token: None,
        };
        assert_eq!(url.link_base_url(), "https://gitea.example.com:3000");
    }

    #[test]
    fn resolve_token_cli_takes_precedence_over_env() {
        temp_env::with_var(
            TokenVar::Gitea.to_string(),
            Some("env_token"),
            || {
                let cli_token = Some(SecretString::from("cli_token"));
                let result = resolve_token(cli_token, None, TokenVar::Gitea);
                assert_eq!(result.unwrap().expose_secret(), "cli_token");
            },
        );
    }

    #[test]
    fn resolve_token_url_takes_precedence_over_env() {
        temp_env::with_var(
            TokenVar::Gitlab.to_string(),
            Some("env_token"),
            || {
                let url_token = SecretString::from("url_token");
                let result =
                    resolve_token(None, Some(&url_token), TokenVar::Gitlab);
                assert_eq!(result.unwrap().expose_secret(), "url_token");
            },
        );
    }
}
