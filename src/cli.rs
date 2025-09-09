use clap::{Parser, Subcommand};
use color_eyre::eyre::{ContextCompat, Result, eyre};
use git_url_parse::GitUrl;
use secrecy::SecretString;
use std::env;

use crate::forge::config::{Remote, RemoteConfig};

/// Program to manage releases! Easily generate changelogs and release PRs
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, default_value = "", global = true)]
    /// Github remote repository
    pub github_repo: String,

    #[arg(long, default_value = "", global = true)]
    /// Github access token
    pub github_token: String,

    #[arg(long, default_value = "", global = true)]
    /// Gitlab remote repository
    pub gitlab_repo: String,

    #[arg(long, default_value = "", global = true)]
    /// Gitlab access token
    pub gitlab_token: String,

    #[arg(long, default_value = "", global = true)]
    /// Gitea remote repository
    pub gitea_repo: String,

    #[arg(long, default_value = "", global = true)]
    /// Gitea access token
    pub gitea_token: String,

    #[arg(long, default_value_t = false, global = true)]
    /// Enables debug logs
    pub debug: bool,

    #[arg(long, default_value_t = false, global = true)]
    /// Enables dry-run mode
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Analyzes commits and creates or updates a release PR
    ReleasePR,
}

impl Args {
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

fn validate_scheme(scheme: git_url_parse::Scheme) -> Result<()> {
    match scheme {
        git_url_parse::Scheme::Http => Ok(()),
        git_url_parse::Scheme::Https => Ok(()),
        _ => Err(eyre!(
            "only http and https schemes are supported for repo urls"
        )),
    }
}

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
            dry_run: true,
            gitea_repo: "".into(),
            gitea_token: "".into(),
            gitlab_repo: "".into(),
            gitlab_token: "".into(),
            github_repo: repo,
            github_token: token,
            command: Command::ReleasePR,
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
            dry_run: true,
            gitea_repo: "".into(),
            gitea_token: "".into(),
            gitlab_repo: repo,
            gitlab_token: token,
            github_repo: "".into(),
            github_token: "".into(),
            command: Command::ReleasePR,
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
            dry_run: true,
            gitea_repo: repo,
            gitea_token: token,
            gitlab_repo: "".into(),
            gitlab_token: "".into(),
            github_repo: "".into(),
            github_token: "".into(),
            command: Command::ReleasePR,
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
            dry_run: true,
            gitea_repo: repo,
            gitea_token: token,
            gitlab_repo: "".into(),
            gitlab_token: "".into(),
            github_repo: "".into(),
            github_token: "".into(),
            command: Command::ReleasePR,
        };

        let result = cli_config.get_remote();
        assert!(result.is_err());
    }
}
