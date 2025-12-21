//! CLI top-level definition for release automation workflow.

use clap::{Parser, Subcommand, ValueEnum};
use color_eyre::eyre::{ContextCompat, eyre};
use git_url_parse::GitUrl;
use secrecy::SecretString;
use std::env;

pub mod command;
pub mod common;
pub mod errors;
pub mod types;

use crate::{
    Result, ShowCommand,
    forge::config::{Remote, RemoteConfig},
};

/// Global CLI arguments for forge configuration and debugging.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    /// Targets a specific forge: github, gitlab, gitea, or local
    #[arg(short, long, value_enum, global = true)]
    forge: Option<ForgeType>,

    /// Repository URL
    #[arg(short, long, global = true)]
    pub repo: Option<String>,

    /// Authentication token. Falls back to env vars: GITHUB_TOKEN, GITLAB_TOKEN, GITEA_TOKEN
    #[arg(short, long, global = true)]
    pub token: Option<String>,

    /// Enable debug logging
    #[arg(long, default_value_t = false, global = true)]
    pub debug: bool,

    /// Execute in dry-run mode
    #[arg(long, default_value_t = false, global = true)]
    pub dry_run: bool,

    /// Base branch for releases. Defaults to repository's default branch
    #[arg(long, global = true)]
    pub base_branch: Option<String>,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ForgeType {
    /// Targets Github as the remote forge
    Github,
    /// Targets Gitlab as the remote forge
    Gitlab,
    /// Targets Gitea as the remote forge
    Gitea,
    /// Targets a local repo for testing / debugging
    Local,
}

/// Release operation subcommands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Analyze commits and create a release pull request
    ReleasePR,

    /// Create a git tag and publish release after PR merge
    Release,

    /// Outputs info about projected and previous releases
    Show {
        #[command(subcommand)]
        command: ShowCommand,
    },
}

impl Cli {
    /// Configure remote repository connection from CLI arguments
    pub fn get_remote(&self) -> Result<Remote> {
        let mut missing = vec![];
        if self.forge.is_none() {
            missing.push("forge")
        }
        if self.repo.is_none() {
            missing.push("repo")
        }

        if !missing.is_empty() {
            let msg = format!("missing required options: {:#?}", missing);
            return Err(eyre!(msg));
        }

        let forge = self.forge.unwrap();
        let repo = self.repo.clone().unwrap();

        match forge {
            ForgeType::Local => Ok(Remote::Local(repo.clone())),
            ForgeType::Github => {
                let config = get_remote_config(
                    forge,
                    &repo,
                    self.token.clone(),
                    self.dry_run,
                )?;
                Ok(Remote::Github(config))
            }
            ForgeType::Gitlab => {
                let config = get_remote_config(
                    forge,
                    &repo,
                    self.token.clone(),
                    self.dry_run,
                )?;
                Ok(Remote::Gitlab(config))
            }
            ForgeType::Gitea => {
                let config = get_remote_config(
                    forge,
                    &repo,
                    self.token.clone(),
                    self.dry_run,
                )?;
                Ok(Remote::Gitea(config))
            }
        }
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

fn get_remote_config(
    forge: ForgeType,
    repo: &str,
    token: Option<String>,
    dry_run: bool,
) -> Result<RemoteConfig> {
    let parsed = GitUrl::parse(repo)?;

    validate_scheme(parsed.scheme)?;

    let mut token = token.unwrap_or_default();

    if token.is_empty()
        && let Some(parsed_token) = parsed.token
    {
        token = parsed_token;
    }

    if token.is_empty() {
        match forge {
            ForgeType::Github => {
                if let Ok(value) = env::var("GITHUB_TOKEN") {
                    token = value;
                }
            }
            ForgeType::Gitlab => {
                if let Ok(value) = env::var("GITLAB_TOKEN") {
                    token = value;
                }
            }
            ForgeType::Gitea => {
                if let Ok(value) = env::var("GITEA_TOKEN") {
                    token = value;
                }
            }
            _ => {}
        }
    }

    if token.is_empty() {
        return Err(eyre!("must set token"));
    }

    let host = parsed.host.ok_or(eyre!("unable to parse host from repo"))?;

    let owner = parsed
        .owner
        .ok_or(eyre!("unable to parse owner from repo"))?;

    let project_path = parsed
        .path
        .strip_prefix("/")
        .wrap_err("failed to process project path")?
        .to_string();

    // github
    let link_base_url = format!("{}://{}", parsed.scheme, host);

    let release_link_base_url = match forge {
        ForgeType::Github => {
            format!("{}/{}/{}/releases/tag", link_base_url, owner, parsed.name)
        }
        ForgeType::Gitlab => {
            format!("{}/{}/{}/-/releases", link_base_url, owner, parsed.name)
        }
        ForgeType::Gitea => {
            format!("{}/{}/{}/releases", link_base_url, owner, parsed.name)
        }
        ForgeType::Local => "".into(),
    };

    Ok(RemoteConfig {
        host,
        port: parsed.port,
        scheme: parsed.scheme.to_string(),
        owner,
        repo: parsed.name,
        path: project_path,
        release_link_base_url,
        token: SecretString::from(token),
        dry_run,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gets_github_remote() {
        let repo = "https://github.com/github_owner/github_repo".to_string();
        let token = "github_token".to_string();

        let cli_config = Cli {
            debug: true,
            dry_run: false,
            forge: Some(ForgeType::Github),
            repo: Some(repo),
            token: Some(token),
            base_branch: None,
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

        let cli_config = Cli {
            debug: true,
            dry_run: false,
            forge: Some(ForgeType::Gitlab),
            repo: Some(repo),
            token: Some(token),
            base_branch: None,
            command: Command::Release,
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

        let cli_config = Cli {
            debug: true,
            dry_run: false,
            forge: Some(ForgeType::Gitea),
            repo: Some(repo),
            token: Some(token),
            base_branch: None,
            command: Command::Show {
                command: ShowCommand::NextRelease {
                    out_file: None,
                    package: None,
                },
            },
        };

        let result = cli_config.get_remote();
        assert!(result.is_ok());

        let remote = result.unwrap();

        assert!(matches!(remote, Remote::Gitea(_)));
    }

    #[test]
    fn gets_local_remote() {
        let repo = ".".to_string();

        let cli_config = Cli {
            debug: true,
            dry_run: false,
            forge: Some(ForgeType::Local),
            repo: Some(repo),
            token: None,
            base_branch: None,
            command: Command::ReleasePR,
        };

        let result = cli_config.get_remote();
        assert!(result.is_ok());

        let remote = result.unwrap();

        assert!(matches!(remote, Remote::Local(_)));
    }

    #[test]
    fn only_supports_http_and_https_schemes() {
        let repo = "git@gitea.com:gitea_owner/gitea_repo".to_string();
        let token = "gitea_token".to_string();

        let cli_config = Cli {
            debug: true,
            dry_run: false,
            forge: Some(ForgeType::Gitea),
            repo: Some(repo),
            token: Some(token),
            base_branch: None,
            command: Command::Show {
                command: ShowCommand::ReleaseNotes {
                    out_file: None,
                    tag: "v1.0.0".into(),
                },
            },
        };

        let result = cli_config.get_remote();
        assert!(result.is_err());
    }
}
