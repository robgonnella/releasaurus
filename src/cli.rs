//! CLI argument parsing and forge platform configuration.
use clap::{Parser, Subcommand};
use color_eyre::eyre::{ContextCompat, eyre};
use git_url_parse::GitUrl;
use secrecy::SecretString;
use std::env;

use crate::{
    forge::config::{Remote, RemoteConfig},
    result::Result,
};

/// Global CLI arguments for forge configuration and debugging.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// GitHub repository URL (https://github.com/owner/repo).
    #[arg(long, default_value = "", global = true)]
    pub github_repo: String,

    /// GitHub personal access token. Falls back to GITHUB_TOKEN env var.
    #[arg(long, default_value = "", global = true)]
    pub github_token: String,

    /// GitLab repository URL. Supports GitLab.com and self-hosted instances.
    #[arg(long, default_value = "", global = true)]
    pub gitlab_repo: String,

    /// GitLab personal access token. Falls back to GITLAB_TOKEN env var.
    #[arg(long, default_value = "", global = true)]
    pub gitlab_token: String,

    /// Gitea repository URL for self-hosted instances.
    #[arg(long, default_value = "", global = true)]
    pub gitea_repo: String,

    /// Gitea access token. Falls back to GITEA_TOKEN env var.
    #[arg(long, default_value = "", global = true)]
    pub gitea_token: String,

    /// Enable debug logging.
    #[arg(long, default_value_t = false, global = true)]
    pub debug: bool,

    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Command,
}

/// Release operation subcommands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Analyze commits and create a release pull request.
    ReleasePR {
        /// Prerelease identifier (e.g., "alpha", "beta", "rc").
        /// Overrides config file setting.
        #[arg(long)]
        prerelease: Option<String>,
    },

    /// Create a git tag and publish release after PR merge.
    Release {
        /// Prerelease identifier (e.g., "alpha", "beta", "rc").
        /// Overrides config file setting.
        #[arg(long)]
        prerelease: Option<String>,
    },
}

impl Args {
    /// Configure remote repository connection from CLI arguments.
    pub fn get_remote(&self) -> Result<Remote> {
        if !self.github_repo.is_empty() {
            return get_github_remote(&self.github_repo, &self.github_token);
        }

        if !self.gitlab_repo.is_empty() {
            return get_gitlab_remote(&self.gitlab_repo, &self.gitlab_token);
        }

        if !self.gitea_repo.is_empty() {
            return get_gitea_remote(&self.gitea_repo, &self.gitea_token);
        }

        Err(eyre!("must configure a remote"))
    }
}

/// Validate that repository URL uses HTTP or HTTPS scheme, rejecting SSH and
/// other protocols.
fn validate_scheme(scheme: git_url_parse::Scheme) -> Result<()> {
    match scheme {
        git_url_parse::Scheme::Http => Ok(()),
        git_url_parse::Scheme::Https => Ok(()),
        _ => Err(eyre!(
            "only http and https schemes are supported for repo urls"
        )),
    }
}

/// Configure GitHub remote by parsing repository URL and resolving
/// authentication token from CLI args or environment.
fn get_github_remote(github_repo: &str, github_token: &str) -> Result<Remote> {
    let parsed = GitUrl::parse(github_repo)?;

    validate_scheme(parsed.scheme)?;

    let mut token = github_token.to_string();

    if token.is_empty()
        && let Some(parsed_token) = parsed.token
    {
        token = parsed_token;
    }

    if token.is_empty()
        && let Ok(env_var_token) = env::var("GITHUB_TOKEN")
    {
        token = env_var_token;
    }

    if token.is_empty() {
        return Err(eyre!("must set github token"));
    }

    let host = parsed
        .host
        .ok_or(eyre!("unable to parse host from github repo"))?;

    let owner = parsed
        .owner
        .ok_or(eyre!("unable to parse owner from gitea repo"))?;

    let project_path = parsed
        .path
        .strip_prefix("/")
        .wrap_err("failed to process project path")?
        .to_string();

    let link_base_url = format!("{}://{}", parsed.scheme, host);

    let commit_link_base_url =
        format!("{}/{}/{}/commit", link_base_url, owner, parsed.name);

    let release_link_base_url =
        format!("{}/{}/{}/releases/tag", link_base_url, owner, parsed.name);

    let remote_config = RemoteConfig {
        host,
        port: parsed.port,
        scheme: parsed.scheme.to_string(),
        owner,
        repo: parsed.name,
        path: project_path,
        commit_link_base_url,
        release_link_base_url,
        token: SecretString::from(token),
    };

    Ok(Remote::Github(remote_config.clone()))
}

/// Configure GitLab remote by parsing repository URL and resolving
/// authentication token from CLI args or environment.
fn get_gitlab_remote(gitlab_repo: &str, gitlab_token: &str) -> Result<Remote> {
    let parsed = GitUrl::parse(gitlab_repo)?;

    validate_scheme(parsed.scheme)?;

    let mut token = gitlab_token.to_string();

    if token.is_empty()
        && let Some(parsed_token) = parsed.token
    {
        token = parsed_token;
    }

    if token.is_empty()
        && let Ok(env_var_token) = env::var("GITLAB_TOKEN")
    {
        token = env_var_token;
    }

    if token.is_empty() {
        return Err(eyre!("must set gitlab token"));
    }

    let host = parsed
        .host
        .ok_or(eyre!("unable to parse host from gitlab repo"))?;

    let owner = parsed
        .owner
        .ok_or(eyre!("unable to parse owner from gitea repo"))?;

    let project_path = parsed
        .path
        .strip_prefix("/")
        .wrap_err("failed to process project path")?
        .to_string();

    let link_base_url = format!("{}://{}", parsed.scheme, host);

    let commit_link_base_url =
        format!("{}/{}/{}/commit", link_base_url, owner, parsed.name);

    let release_link_base_url =
        format!("{}/{}/{}/releases", link_base_url, owner, parsed.name);

    let remote_config = RemoteConfig {
        host,
        port: parsed.port,
        scheme: parsed.scheme.to_string(),
        owner,
        repo: parsed.name,
        path: project_path,
        commit_link_base_url,
        release_link_base_url,
        token: SecretString::from(token),
    };

    Ok(Remote::Gitlab(remote_config.clone()))
}

/// Configure Gitea remote by parsing repository URL and resolving
/// authentication token from CLI args or environment.
fn get_gitea_remote(gitea_repo: &str, gitea_token: &str) -> Result<Remote> {
    let parsed = GitUrl::parse(gitea_repo)?;

    validate_scheme(parsed.scheme)?;

    let mut token = gitea_token.to_string();

    if token.is_empty()
        && let Some(parsed_token) = parsed.token
    {
        token = parsed_token;
    }

    if token.is_empty()
        && let Ok(env_var_token) = env::var("GITEA_TOKEN")
    {
        token = env_var_token;
    }

    if token.is_empty() {
        return Err(eyre!("must set gitea token"));
    }

    let host = parsed
        .host
        .ok_or(eyre!("unable to parse host from gitea repo"))?;

    let owner = parsed
        .owner
        .ok_or(eyre!("unable to parse owner from gitea repo"))?;

    let project_path = parsed
        .path
        .strip_prefix("/")
        .wrap_err("failed to process project path")?
        .to_string();

    let link_base_url = format!("{}://{}", parsed.scheme, host);

    let commit_link_base_url =
        format!("{}/{}/{}/commit", link_base_url, owner, parsed.name);

    let release_link_base_url =
        format!("{}/{}/{}/releases", link_base_url, owner, parsed.name);

    let remote_config = RemoteConfig {
        host,
        port: parsed.port,
        scheme: parsed.scheme.to_string(),
        owner,
        repo: parsed.name,
        path: project_path,
        commit_link_base_url,
        release_link_base_url,
        token: SecretString::from(token),
    };

    Ok(Remote::Gitea(remote_config.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gets_github_remote() {
        let repo = "https://github.com/github_owner/github_repo".to_string();
        let token = "github_token".to_string();

        let cli_config = Args {
            debug: true,
            gitea_repo: "".into(),
            gitea_token: "".into(),
            gitlab_repo: "".into(),
            gitlab_token: "".into(),
            github_repo: repo,
            github_token: token,
            command: Command::ReleasePR { prerelease: None },
        };

        let result = cli_config.get_remote();
        assert!(result.is_ok());

        let remote = result.unwrap();

        assert!(matches!(remote, Remote::Github(_)));
    }

    #[test]
    fn gets_gitlab_remote() {
        let repo = "https://gitlab.com/gitlab_owner/gitlab_repo".to_string();
        let token = "gitlab_token".to_string();

        let cli_config = Args {
            debug: true,
            gitea_repo: "".into(),
            gitea_token: "".into(),
            gitlab_repo: repo,
            gitlab_token: token,
            github_repo: "".into(),
            github_token: "".into(),
            command: Command::ReleasePR { prerelease: None },
        };

        let result = cli_config.get_remote();
        assert!(result.is_ok());

        let remote = result.unwrap();

        assert!(matches!(remote, Remote::Gitlab(_)));
    }

    #[test]
    fn gets_gitea_remote() {
        let repo = "http://gitea.com/gitea_owner/gitea_repo".to_string();
        let token = "gitea_token".to_string();

        let cli_config = Args {
            debug: true,
            gitea_repo: repo,
            gitea_token: token,
            gitlab_repo: "".into(),
            gitlab_token: "".into(),
            github_repo: "".into(),
            github_token: "".into(),
            command: Command::ReleasePR { prerelease: None },
        };

        let result = cli_config.get_remote();
        assert!(result.is_ok());

        let remote = result.unwrap();

        assert!(matches!(remote, Remote::Gitea(_)));
    }

    #[test]
    fn only_supports_http_and_https_schemes() {
        let repo = "git@gitea.com:gitea_owner/gitea_repo".to_string();
        let token = "gitea_token".to_string();

        let cli_config = Args {
            debug: true,
            gitea_repo: repo,
            gitea_token: token,
            gitlab_repo: "".into(),
            gitlab_token: "".into(),
            github_repo: "".into(),
            github_token: "".into(),
            command: Command::ReleasePR { prerelease: None },
        };

        let result = cli_config.get_remote();
        assert!(result.is_err());
    }
}
