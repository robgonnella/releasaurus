//! Command Line Interface (CLI) definition and argument parsing.
//!
//! This module defines the complete CLI interface for Releasaurus, including:
//! - Global arguments for forge configuration and debugging
//! - Subcommands for different release operations
//! - Remote repository configuration parsing and validation
//! - Token authentication from multiple sources (CLI args, URLs, environment variables)
//!
//! The CLI supports multiple forge platforms (GitHub, GitLab, Gitea) and provides
//! flexible authentication options for seamless integration into various workflows.
use clap::{Parser, Subcommand};
use color_eyre::eyre::{ContextCompat, eyre};
use git_url_parse::GitUrl;
use secrecy::SecretString;
use std::env;

use crate::{
    forge::config::{Remote, RemoteConfig},
    result::Result,
};

/// Main command line arguments structure for Releasaurus.
///
/// This structure defines all global CLI arguments that are available across all subcommands.
/// It supports configuration for multiple forge platforms and provides common options
/// for debugging and dry-run execution.
///
/// # Authentication Priority
///
/// For each forge platform, authentication tokens are resolved in this order:
/// 1. Explicitly provided CLI argument (--github-token, --gitlab-token, --gitea-token)
/// 2. Environment variable (GITHUB_TOKEN, GITLAB_TOKEN, GITEA_TOKEN)
///
/// # Examples
///
/// ```bash
/// # Using GitHub with token from environment
/// export GITHUB_TOKEN=ghp_xxx
/// releasaurus --github-repo https://github.com/user/repo release-pr
///
/// # Using GitLab with explicit token
/// releasaurus --gitlab-repo https://gitlab.com/user/repo --gitlab-token glpat-xxx release
/// ```
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, default_value = "", global = true)]
    /// GitHub remote repository URL.
    ///
    /// Specifies the GitHub repository to operate on. Must be a complete HTTPS or HTTP URL.
    ///
    /// Examples:
    /// - `https://github.com/owner/repo`
    pub github_repo: String,

    #[arg(long, default_value = "", global = true)]
    /// GitHub personal access token for authentication.
    ///
    /// Required for GitHub operations. If not provided, the system will attempt
    /// to use a token from the GITHUB_TOKEN environment variable.
    ///
    /// The token must have appropriate permissions:
    /// - `repo` scope for private repositories
    /// - `public_repo` scope for public repositories
    /// - `write:packages` if publishing packages
    pub github_token: String,

    #[arg(long, default_value = "", global = true)]
    /// GitLab remote repository URL.
    ///
    /// Specifies the GitLab repository to operate on. Must be a complete HTTPS or HTTP URL.
    /// Supports both GitLab.com and self-hosted GitLab instances.
    ///
    /// Examples:
    /// - `https://gitlab.com/owner/repo`
    /// - `https://gitlab.example.com/group/subgroup/repo`
    pub gitlab_repo: String,

    #[arg(long, default_value = "", global = true)]
    /// GitLab personal access token for authentication.
    ///
    /// Required for GitLab operations. If not provided, the system will attempt
    /// to use a token from the GITLAB_TOKEN environment variable.
    ///
    /// The token must have appropriate scopes:
    /// - `api` for full API access
    /// - `read_repository` and `write_repository` for repository operations
    pub gitlab_token: String,

    #[arg(long, default_value = "", global = true)]
    /// Gitea remote repository URL.
    ///
    /// Specifies the Gitea repository to operate on. Must be a complete HTTPS or HTTP URL.
    /// Supports self-hosted Gitea instances.
    ///
    /// Examples:
    /// - `https://gitea.com/owner/repo`
    /// - `https://git.example.com/organization/repo`
    pub gitea_repo: String,

    #[arg(long, default_value = "", global = true)]
    /// Gitea access token for authentication.
    ///
    /// Required for Gitea operations. If not provided, the system will attempt
    /// to use a token from the GITEA_TOKEN environment variable.
    ///
    /// The token must have repository read/write permissions.
    pub gitea_token: String,

    #[arg(long, default_value = "250", global = true)]
    /// Sets the clone depth
    ///
    /// Use 0 to clone all history
    pub clone_depth: u64,

    #[arg(long, default_value_t = false, global = true)]
    /// Enable debug-level logging output.
    ///
    /// When enabled, shows detailed debug information including:
    /// - HTTP requests and responses
    /// - Git operations and status
    /// - File system operations
    /// - Version detection logic
    pub debug: bool,

    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Command,
}

/// Available subcommands for release operations.
///
/// Each command represents a different stage of the release process,
/// from preparation to final publication.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Analyze commits and create or update a release pull request.
    ///
    /// This command:
    /// 1. Analyzes commits since the last release using git-cliff
    /// 2. Determines the next semantic version
    /// 3. Updates version files across supported languages/frameworks
    /// 4. Generates or updates a changelog
    /// 5. Creates a pull request with all changes
    ///
    /// The pull request serves as a review stage before the final release.
    ReleasePR,

    /// Process the release commit and create a release in the configured forge.
    ///
    /// This command:
    /// 1. Validates that we're on a release commit
    /// 2. Creates a git tag for the release
    /// 3. Pushes the tag to the remote repository
    /// 4. Creates a release entry in the forge platform
    ///
    /// This should be run after a release PR has been merged.
    Release,
}

impl Args {
    /// Determine and configure the remote repository based on provided arguments.
    ///
    /// This method examines the CLI arguments to determine which forge platform
    /// to use and configures the appropriate remote connection. It prioritizes
    /// GitHub over GitLab over Gitea if multiple are configured.
    ///
    /// # Returns
    ///
    /// * `Result<Remote>` - Configured remote connection or error if none specified
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No remote repository is configured
    /// - Repository URL parsing fails
    /// - Authentication token is missing or invalid
    /// - Repository URL uses unsupported scheme (only HTTP/HTTPS supported)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let args = Args::parse();
    /// let remote = args.get_remote()?;
    /// match remote {
    ///     Remote::Github(config) => println!("Using GitHub: {}", config.repo),
    ///     Remote::Gitlab(config) => println!("Using GitLab: {}", config.repo),
    ///     Remote::Gitea(config) => println!("Using Gitea: {}", config.repo),
    /// }
    /// ```
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

/// Validate that the repository URL uses a supported scheme.
///
/// Only HTTP and HTTPS schemes are supported for forge API compatibility.
/// SSH URLs (git@) are not supported as they don't work with REST APIs.
///
/// # Arguments
///
/// * `scheme` - The URL scheme parsed from the repository URL
///
/// # Returns
///
/// * `Result<()>` - Ok if scheme is supported, Err otherwise
///
/// # Errors
///
/// Returns an error if the scheme is not HTTP or HTTPS.
fn validate_scheme(scheme: git_url_parse::Scheme) -> Result<()> {
    match scheme {
        git_url_parse::Scheme::Http => Ok(()),
        git_url_parse::Scheme::Https => Ok(()),
        _ => Err(eyre!(
            "only http and https schemes are supported for repo urls"
        )),
    }
}

/// Configure a GitHub remote repository connection.
///
/// Parses the GitHub repository URL, validates the scheme, and configures
/// authentication. Constructs the necessary URLs for commit and release linking.
///
/// # Arguments
///
/// * `github_repo` - GitHub repository URL
/// * `github_token` - GitHub personal access token (may be empty)
///
/// # Returns
///
/// * `Result<Remote>` - Configured GitHub remote or error
///
/// # Errors
///
/// Returns an error if:
/// - URL parsing fails
/// - Scheme is not HTTP/HTTPS
/// - No authentication token is available
/// - Required URL components (host, owner) are missing
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

/// Configure a GitLab remote repository connection.
///
/// Parses the GitLab repository URL, validates the scheme, and configures
/// authentication. Supports both GitLab.com and self-hosted instances.
///
/// # Arguments
///
/// * `gitlab_repo` - GitLab repository URL
/// * `gitlab_token` - GitLab personal access token (may be empty)
///
/// # Returns
///
/// * `Result<Remote>` - Configured GitLab remote or error
///
/// # Errors
///
/// Returns an error if:
/// - URL parsing fails
/// - Scheme is not HTTP/HTTPS
/// - No authentication token is available
/// - Required URL components (host, owner) are missing
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

/// Configure a Gitea remote repository connection.
///
/// Parses the Gitea repository URL, validates the scheme, and configures
/// authentication. Supports self-hosted Gitea instances.
///
/// # Arguments
///
/// * `gitea_repo` - Gitea repository URL
/// * `gitea_token` - Gitea access token (may be empty)
///
/// # Returns
///
/// * `Result<Remote>` - Configured Gitea remote or error
///
/// # Errors
///
/// Returns an error if:
/// - URL parsing fails
/// - Scheme is not HTTP/HTTPS
/// - No authentication token is available
/// - Required URL components (host, owner) are missing
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
    //! Unit tests for CLI argument parsing and remote configuration.
    use super::*;

    /// Test GitHub remote configuration from CLI arguments.
    #[test]
    fn gets_github_remote() {
        let repo = "https://github.com/github_owner/github_repo".to_string();
        let token = "github_token".to_string();

        let cli_config = Args {
            debug: true,
            clone_depth: 0,
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

    /// Test GitLab remote configuration from CLI arguments.
    #[test]
    fn gets_gitlab_remote() {
        let repo = "https://gitlab.com/gitlab_owner/gitlab_repo".to_string();
        let token = "gitlab_token".to_string();

        let cli_config = Args {
            debug: true,
            clone_depth: 0,
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

    /// Test Gitea remote configuration from CLI arguments.
    #[test]
    fn gets_gitea_remote() {
        let repo = "http://gitea.com/gitea_owner/gitea_repo".to_string();
        let token = "gitea_token".to_string();

        let cli_config = Args {
            debug: true,
            clone_depth: 0,
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

    /// Test that only HTTP and HTTPS schemes are supported for repository URLs.
    #[test]
    fn only_supports_http_and_https_schemes() {
        let repo = "git@gitea.com:gitea_owner/gitea_repo".to_string();
        let token = "gitea_token".to_string();

        let cli_config = Args {
            debug: true,
            clone_depth: 0,
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
