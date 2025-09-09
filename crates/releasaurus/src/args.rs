use clap::{Parser, Subcommand};
use color_eyre::eyre::{Result, eyre};
use git_url_parse::GitUrl;
use releasaurus_core::config::{Remote, RemoteConfig};
use secrecy::Secret;
use std::env;

/// Program to manage releases! Easily generate changelogs and release PRs
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
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

    #[arg(long, default_value = None, global = true)]
    /// Optional Api url to use for remote. You should set this if you
    /// are using a non-community based forge like github or gitlab enterprise,
    /// or self-hosted gitea
    pub api_url: Option<String>,

    #[arg(long, default_value_t = false, global = true)]
    /// Enables debug logs
    pub debug: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Analyzes commits and creates or updates a release PR
    ReleasePR,
}

impl Cli {
    pub fn get_remote(&self) -> Result<Remote> {
        if !self.github_repo.is_empty() {
            return get_github_remote(
                self.github_repo.clone(),
                self.github_token.clone(),
                self.api_url.clone(),
            );
        }

        if !self.gitlab_repo.is_empty() {
            return get_gitlab_remote(
                self.gitlab_repo.clone(),
                self.gitlab_token.clone(),
                self.api_url.clone(),
            );
        }

        if !self.gitea_repo.is_empty() {
            return get_gitea_remote(
                self.gitea_repo.clone(),
                self.gitea_token.clone(),
                self.api_url.clone(),
            );
        }

        Err(eyre!("must configure a remote"))
    }
}

fn get_github_remote(
    github_repo: String,
    github_token: String,
    api_url: Option<String>,
) -> Result<Remote> {
    let parsed = GitUrl::parse(github_repo.as_str())?;
    let mut token = github_token;

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

    let mut scheme = "https".to_string();

    if let git_url_parse::Scheme::Http = parsed.scheme {
        scheme = "http".to_string();
    }

    let link_base_url = format!("{}://{}", scheme, host);

    let commit_link_base_url =
        format!("{}/{}/{}/commit", link_base_url, owner, parsed.name);

    let release_link_base_url =
        format!("{}/{}/{}/releases/tag", link_base_url, owner, parsed.name);

    Ok(Remote::Github(RemoteConfig {
        host,
        scheme,
        owner,
        repo: parsed.name,
        commit_link_base_url,
        release_link_base_url,
        api_url,
        token: Secret::new(token),
    }))
}

fn get_gitlab_remote(
    gitlab_repo: String,
    gitlab_token: String,
    api_url: Option<String>,
) -> Result<Remote> {
    let parsed = GitUrl::parse(gitlab_repo.as_str())?;
    let mut token = gitlab_token;

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

    let mut scheme = "https".to_string();

    if let git_url_parse::Scheme::Http = parsed.scheme {
        scheme = "http".to_string();
    }

    let link_base_url = format!("{}://{}", scheme, host);

    let commit_link_base_url =
        format!("{}/{}/{}/commit", link_base_url, owner, parsed.name);

    let release_link_base_url =
        format!("{}/{}/{}/releases", link_base_url, owner, parsed.name);

    Ok(Remote::Gitlab(RemoteConfig {
        host,
        scheme,
        owner,
        repo: parsed.name,
        commit_link_base_url,
        release_link_base_url,
        api_url,
        token: Secret::new(token),
    }))
}

fn get_gitea_remote(
    gitea_repo: String,
    gitea_token: String,
    api_url: Option<String>,
) -> Result<Remote> {
    let parsed = GitUrl::parse(gitea_repo.as_str())?;
    let mut token = gitea_token;

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

    let mut scheme = "https".to_string();

    if let git_url_parse::Scheme::Http = parsed.scheme {
        scheme = "http".to_string();
    }

    let link_base_url = format!("{}://{}", scheme, host);

    let commit_link_base_url =
        format!("{}/{}/{}/commit", link_base_url, owner, parsed.name);

    let release_link_base_url =
        format!("{}/{}/{}/releases", link_base_url, owner, parsed.name);

    Ok(Remote::Gitea(RemoteConfig {
        host,
        scheme,
        owner,
        repo: parsed.name,
        commit_link_base_url,
        release_link_base_url,
        api_url,
        token: Secret::new(token),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gets_github_remote() {
        let mut cli_config = Cli::parse();
        let repo = "https://github.com/github_owner/github_repo".to_string();
        let token = "github_token".to_string();

        cli_config.github_repo = repo;
        cli_config.github_token = token;

        let result = cli_config.get_remote();
        assert!(result.is_ok());

        let remote = result.unwrap();

        assert!(matches!(remote, Remote::Github(_)));
    }

    #[test]
    fn gets_gitlab_remote() {
        let mut cli_config = Cli::parse();
        let repo = "https://gitlab.com/gitlab_owner/gitlab_repo".to_string();
        let token = "gitlab_token".to_string();

        cli_config.gitlab_repo = repo;
        cli_config.gitlab_token = token;

        let result = cli_config.get_remote();
        assert!(result.is_ok());

        let remote = result.unwrap();

        assert!(matches!(remote, Remote::Gitlab(_)));
    }

    #[test]
    fn gets_gitea_remote() {
        let mut cli_config = Cli::parse();
        let repo = "https://gitea.com/gitea_owner/gitea_repo".to_string();
        let token = "gitea_token".to_string();

        cli_config.gitea_repo = repo;
        cli_config.gitea_token = token;

        let result = cli_config.get_remote();
        assert!(result.is_ok());

        let remote = result.unwrap();

        assert!(matches!(remote, Remote::Gitea(_)));
    }
}
