//! CLI argument parsing and forge platform configuration.
use clap::{Parser, Subcommand};
use color_eyre::eyre::{ContextCompat, Result as EyreResult, eyre};
use git_url_parse::GitUrl;
use secrecy::SecretString;
use std::{env, fmt};

use crate::{
    analyzer::release::Release,
    config::{ManifestFile, ReleaseType},
    forge::config::{Remote, RemoteConfig},
};

/// Type alias for Result with color-eyre error reporting and diagnostics.
pub type Result<T> = EyreResult<T>;

/// Global CLI arguments for forge configuration and debugging.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// GitHub repository URL (<https://github.com/owner/repo>).
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

    /// Local repository path. For testing config changes against local repo
    #[arg(long, default_value = "", global = true)]
    pub local_repo: String,

    /// Enable debug logging.
    #[arg(long, default_value_t = false, global = true)]
    pub debug: bool,

    /// Execute in dry-run mode
    #[arg(long, default_value_t = false, global = true)]
    pub dry_run: bool,

    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Command,
}

/// Release operation subcommands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Analyze commits and create a release pull request.
    ReleasePR,

    /// Create a git tag and publish release after PR merge.
    Release,
}

impl Args {
    /// Configure remote repository connection from CLI arguments.
    pub fn get_remote(&self) -> Result<Remote> {
        if !self.local_repo.is_empty() {
            return Ok(Remote::Local(self.local_repo.clone()));
        }

        if !self.github_repo.is_empty() {
            return get_github_remote(
                &self.github_repo,
                &self.github_token,
                self.dry_run,
            );
        }

        if !self.gitlab_repo.is_empty() {
            return get_gitlab_remote(
                &self.gitlab_repo,
                &self.gitlab_token,
                self.dry_run,
            );
        }

        if !self.gitea_repo.is_empty() {
            return get_gitea_remote(
                &self.gitea_repo,
                &self.gitea_token,
                self.dry_run,
            );
        }

        Err(eyre!("must configure a remote"))
    }
}

/// Represents a release-able package in manifest
#[derive(Debug)]
pub struct ReleasablePackage {
    /// The name of this package
    pub name: String,
    /// Path to package directory relative to workspace_root path
    pub path: String,
    /// Path to the workspace root directory for this package relative to the repository root
    pub workspace_root: String,
    /// The [`ReleaseType`] for this package
    pub release_type: ReleaseType,
    /// The computed Release for this package
    pub release: Release,
    /// Additional version manifest files to apply updates to
    pub additional_manifest_files: Option<Vec<ManifestFile>>,
}

/// Error indicating a pending release that hasn't been tagged yet.
///
/// This error is returned when attempting to create a new release PR
/// while a previous release PR has been merged but not yet tagged.
#[derive(Debug, Clone)]
pub struct PendingReleaseError {
    /// The release branch that has a pending release
    pub branch: String,
    /// The PR number of the pending release
    pub pr_number: u64,
}

impl fmt::Display for PendingReleaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "found pending release (PR #{}) on branch '{}' that has not been tagged yet: \
             cannot continue, must finish previous release first",
            self.pr_number, self.branch
        )
    }
}

impl std::error::Error for PendingReleaseError {}

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
fn get_github_remote(
    github_repo: &str,
    github_token: &str,
    dry_run: bool,
) -> Result<Remote> {
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

    let release_link_base_url =
        format!("{}/{}/{}/releases/tag", link_base_url, owner, parsed.name);

    Ok(Remote::Github(RemoteConfig {
        host,
        port: parsed.port,
        scheme: parsed.scheme.to_string(),
        owner,
        repo: parsed.name,
        path: project_path,
        release_link_base_url,
        token: SecretString::from(token),
        dry_run,
    }))
}

/// Configure GitLab remote by parsing repository URL and resolving
/// authentication token from CLI args or environment.
fn get_gitlab_remote(
    gitlab_repo: &str,
    gitlab_token: &str,
    dry_run: bool,
) -> Result<Remote> {
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

    let release_link_base_url =
        format!("{}/{}/{}/-/releases", link_base_url, owner, parsed.name);

    Ok(Remote::Gitlab(RemoteConfig {
        host,
        port: parsed.port,
        scheme: parsed.scheme.to_string(),
        owner,
        repo: parsed.name,
        path: project_path,
        release_link_base_url,
        token: SecretString::from(token),
        dry_run,
    }))
}

/// Configure Gitea remote by parsing repository URL and resolving
/// authentication token from CLI args or environment.
fn get_gitea_remote(
    gitea_repo: &str,
    gitea_token: &str,
    dry_run: bool,
) -> Result<Remote> {
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

    let release_link_base_url =
        format!("{}/{}/{}/releases", link_base_url, owner, parsed.name);

    Ok(Remote::Gitea(RemoteConfig {
        host,
        port: parsed.port,
        scheme: parsed.scheme.to_string(),
        owner,
        repo: parsed.name,
        path: project_path,
        release_link_base_url,
        token: SecretString::from(token),
        dry_run,
    }))
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
            local_repo: "".into(),
            gitea_repo: "".into(),
            gitea_token: "".into(),
            gitlab_repo: "".into(),
            gitlab_token: "".into(),
            github_repo: repo,
            github_token: token,
            dry_run: false,
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
            local_repo: "".into(),
            gitea_repo: "".into(),
            gitea_token: "".into(),
            gitlab_repo: repo,
            gitlab_token: token,
            github_repo: "".into(),
            github_token: "".into(),
            dry_run: false,
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
            local_repo: "".into(),
            gitea_repo: repo,
            gitea_token: token,
            gitlab_repo: "".into(),
            gitlab_token: "".into(),
            github_repo: "".into(),
            github_token: "".into(),
            dry_run: false,
            command: Command::ReleasePR,
        };

        let result = cli_config.get_remote();
        assert!(result.is_ok());

        let remote = result.unwrap();

        assert!(matches!(remote, Remote::Gitea(_)));
    }

    #[test]
    fn gets_local_remote() {
        let repo = ".".to_string();

        let cli_config = Args {
            debug: true,
            local_repo: repo,
            gitea_repo: "".into(),
            gitea_token: "".into(),
            gitlab_repo: "".into(),
            gitlab_token: "".into(),
            github_repo: "".into(),
            github_token: "".into(),
            command: Command::ReleasePR,
            dry_run: false,
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

        let cli_config = Args {
            debug: true,
            local_repo: "".into(),
            gitea_repo: repo,
            gitea_token: token,
            gitlab_repo: "".into(),
            gitlab_token: "".into(),
            github_repo: "".into(),
            github_token: "".into(),
            dry_run: false,
            command: Command::ReleasePR,
        };

        let result = cli_config.get_remote();
        assert!(result.is_err());
    }

    #[test]
    fn test_pending_release_error_into_eyre() {
        let error = PendingReleaseError {
            branch: "test-branch".to_string(),
            pr_number: 55,
        };

        // Test that it can be converted into color_eyre::eyre::Error
        let eyre_error: color_eyre::eyre::Error = error.into();
        let error_string = format!("{}", eyre_error);
        assert!(error_string.contains("PR #55"));
        assert!(error_string.contains("test-branch"));
    }
}
