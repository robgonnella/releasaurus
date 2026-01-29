//! CLI top-level definition for release automation workflow.

use clap::{Args, Parser, Subcommand, ValueEnum};
use color_eyre::eyre::ContextCompat;
use git_url_parse::GitUrl;
use merge::Merge;
use secrecy::SecretString;
use serde::Deserialize;
use std::{collections::HashMap, env};

pub mod get;

use crate::{
    Result,
    config::{changelog::RewordedCommit, prerelease::PrereleaseStrategy},
    error::ReleasaurusError,
    forge::config::{Remote, RemoteConfig},
};

/// Global CLI arguments for forge configuration and debugging.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[command(flatten)]
    pub forge_args: ForgeArgs,

    /// Enable debug logging
    #[arg(long, default_value_t = false, global = true)]
    pub debug: bool,

    /// Base branch for releases. Defaults to repository's default branch
    #[arg(long, global = true)]
    pub base_branch: Option<String>,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Args)]
pub struct ForgeArgs {
    /// Targets a specific forge: github, gitlab, gitea, or local
    #[arg(short, long, value_enum, global = true)]
    pub forge: Option<ForgeType>,

    /// Repository URL
    #[arg(short, long, global = true)]
    pub repo: Option<String>,

    /// Authentication token. Falls back to env vars: GITHUB_TOKEN, GITLAB_TOKEN, GITEA_TOKEN
    #[arg(short, long, global = true)]
    pub token: Option<String>,
}

impl ForgeArgs {
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
            return Err(ReleasaurusError::invalid_config(msg));
        }

        let forge = self.forge.unwrap();
        let repo = self.repo.clone().unwrap();

        match forge {
            ForgeType::Local => Ok(Remote::Local(repo.clone())),
            ForgeType::Github => {
                let config =
                    get_remote_config(forge, &repo, self.token.clone())?;
                Ok(Remote::Github(config))
            }
            ForgeType::Gitlab => {
                let config =
                    get_remote_config(forge, &repo, self.token.clone())?;
                Ok(Remote::Gitlab(config))
            }
            ForgeType::Gitea => {
                let config =
                    get_remote_config(forge, &repo, self.token.clone())?;
                Ok(Remote::Gitea(config))
            }
        }
    }
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

#[derive(Debug, Clone)]
pub struct PackagePathOverride {
    pub package_name: String,
    pub path: String,
    pub value: String,
}

#[derive(Debug, Clone, Merge, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageOverrides {
    #[serde(rename = "tag_prefix")]
    #[merge(strategy = merge::option::overwrite_none)]
    pub tag_prefix: Option<String>,
    #[serde(rename = "prerelease.suffix")]
    #[merge(strategy = merge::option::overwrite_none)]
    pub prerelease_suffix: Option<String>,
    #[serde(rename = "prerelease.strategy")]
    #[merge(strategy = merge::option::overwrite_none)]
    pub prerelease_strategy: Option<PrereleaseStrategy>,
}

#[derive(Debug, Clone, Default, Merge, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalOverrides {
    #[merge(strategy = merge::option::overwrite_none)]
    pub base_branch: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub tag_prefix: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub prerelease_suffix: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub prerelease_strategy: Option<PrereleaseStrategy>,
}

#[derive(Debug, Clone, Args)]
pub struct SharedCommandOverrides {
    /// Override package properties using dot notation
    /// Example: --set-package my-pkg.prerelease.suffix=beta
    #[arg(long = "set-package", value_parser = parse_package_override, value_name = "KEY=VALUE")]
    package_overrides: Vec<PackagePathOverride>,

    /// Global override for tag_prefix. Overrides package config. Can
    /// be overridden via explicit "--set-package" override
    #[arg(long)]
    tag_prefix: Option<String>,

    /// Global override for prerelease suffix. Overrides package config. Can
    /// be overridden via explicit "--set-package" override
    #[arg(long)]
    prerelease_suffix: Option<String>,

    /// Global override for prerelease strategy. Overrides package config. Can
    /// be overridden via explicit "--set-package" override
    #[arg(long, value_enum)]
    prerelease_strategy: Option<PrereleaseStrategy>,
}

#[derive(Debug, Clone, Default, Args)]
pub struct CommitModifiers {
    /// Commit sha (or prefix) to skip when calculating next version and
    /// generating changelog. Matches any commit whose SHA starts with the
    /// provided value. Can be provided more than once to skip multiple commits
    #[arg(long = "skip-sha", value_parser = validate_sha, value_name = "SKIP_SHA")]
    pub skip_shas: Vec<String>,

    /// Rewords a commit message when generating changelog. Must be in the
    /// form "sha=message". The SHA can be a prefix - matches any commit whose
    /// SHA starts with the provided value. Can be provided more than once to
    /// reword multiple commits
    /// Example: --reword "abc123de=fix: a new message\n\nMore content"
    #[arg(long, value_parser = parse_reworded_commit, value_name = "KEY=VALUE")]
    pub reword: Vec<RewordedCommit>,
}

/// Validates that a string is a valid git commit SHA (7-40 hex characters)
pub fn validate_sha(sha: &str) -> Result<String> {
    let trimmed = sha.trim();

    if trimmed.len() < 7 {
        return Err(ReleasaurusError::invalid_config(format!(
            "Invalid commit SHA: '{}'. Must be at least 7 characters",
            sha
        )));
    }

    if trimmed.len() > 40 {
        return Err(ReleasaurusError::invalid_config(format!(
            "Invalid commit SHA: '{}'. Must not exceed 40 characters",
            sha
        )));
    }

    if !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ReleasaurusError::invalid_config(format!(
            "Invalid commit SHA: '{}'. Must contain only hexadecimal characters (0-9, a-f)",
            sha
        )));
    }

    Ok(trimmed.to_string())
}

fn parse_package_override(s: &str) -> Result<PackagePathOverride> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(ReleasaurusError::invalid_config(format!(
            "Invalid format: '{}'. Expected package_name.path=value",
            s
        )));
    }

    let key = parts[0];
    let value = parts[1];
    let key_parts: Vec<&str> = key.split('.').collect();

    if key_parts.len() < 2 {
        return Err(ReleasaurusError::invalid_config(format!(
            "Invalid key: '{}'. Expected package_name.path",
            key
        )));
    }

    Ok(PackagePathOverride {
        package_name: key_parts[0].to_string(),
        // Support nested like "prerelease.suffix"
        path: key_parts[1..].join("."),
        value: value.to_string(),
    })
}

fn parse_reworded_commit(s: &str) -> Result<RewordedCommit> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(ReleasaurusError::invalid_config(format!(
            "Invalid --reword format: '{}'. Expected 'commit_sha=new_message'. Example: --reword 'abc123de=fix: corrected message'",
            s
        )));
    }

    let sha = parts[0];
    let message = parts[1];

    // Validate the commit SHA format and get the trimmed version
    let validated_sha = validate_sha(sha)?;

    Ok(RewordedCommit {
        sha: validated_sha,
        message: message.into(),
    })
}

#[derive(Subcommand, Debug, Clone)]
pub enum GetCommand {
    /// Outputs the projected next release in json
    NextRelease {
        /// Output projected-release json directly to file
        #[arg(short, long)]
        out_file: Option<String>,

        /// Optionally restrict output to just 1 specific package
        #[arg(short, long)]
        package: Option<String>,

        #[command(flatten)]
        commit_modifiers: CommitModifiers,

        #[command(flatten)]
        overrides: SharedCommandOverrides,
    },

    /// Outputs most recent releases
    CurrentRelease {
        /// Output release json data directly to file
        #[arg(short, long)]
        out_file: Option<String>,

        /// Optionally restrict output to just 1 specific package
        #[arg(short, long)]
        package: Option<String>,
    },

    /// Outputs the release data associated with a given tag
    Release {
        /// Output release json data directly to file
        #[arg(short, long)]
        out_file: Option<String>,

        /// Gets release notes associated with specific tag
        #[arg(long, required = true)]
        tag: String,
    },

    /// Ingests json file generated from "get next-release" and converts
    /// from release json to notes json using configured tera template.
    /// Outputs a  json array of package name and associated notes. This
    /// enables the ability to generate json for next release, perform custom
    /// transformations (like replacing author names with slack userIDs), then
    /// recompile into markdown notes.
    #[command(visible_alias = "notes")]
    RecompiledNotes {
        /// The json file generated by "get next-release" to ingest
        #[arg(long)]
        file: String,

        /// Output notes json directly to file
        #[arg(short, long)]
        out_file: Option<String>,
    },
}

/// Release operation subcommands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Analyze commits and create a release pull request
    ReleasePR {
        #[command(flatten)]
        commit_modifiers: CommitModifiers,

        #[command(flatten)]
        overrides: SharedCommandOverrides,

        /// Execute in dry-run mode
        #[arg(long, default_value_t = false, global = true)]
        dry_run: bool,
    },

    /// Create a git tag and publish release after PR merge
    Release {
        /// Execute in dry-run mode
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Outputs info about projected and previous releases
    #[command(visible_alias = "show")]
    Get {
        #[command(subcommand)]
        command: GetCommand,
    },

    /// Performs patch version update in manifest version files to start next
    /// release. This does not create any PRs or perform any tagging. It updates
    /// the version files and commits the changes to the targeted base branch
    /// as a "chore" commit
    StartNext {
        #[command(flatten)]
        overrides: SharedCommandOverrides,

        /// Optional comma separated list of package names to target
        #[arg(long, value_delimiter(','))]
        packages: Option<Vec<String>>,

        /// Execute in dry-run mode
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}

impl Cli {
    pub fn get_commit_modifiers(&self) -> CommitModifiers {
        match &self.command {
            Command::ReleasePR {
                commit_modifiers, ..
            } => commit_modifiers.to_owned(),
            Command::Get {
                command:
                    GetCommand::NextRelease {
                        commit_modifiers, ..
                    },
            } => commit_modifiers.to_owned(),
            _ => CommitModifiers::default(),
        }
    }

    /// Gathers the list of provided path override options, like
    /// --releasaurus.prerelease.suffix=beta, and collects them into a single
    /// struct of all allowed overrides properties for each package name.
    /// Returns HashMap<pkg_name, Option<PackageOverrides>>
    pub fn get_package_overrides(
        &self,
    ) -> Result<HashMap<String, PackageOverrides>> {
        let mut map: HashMap<String, PackageOverrides> = HashMap::new();

        let mut map_overrides =
            |overrides: &SharedCommandOverrides| -> Result<()> {
                for path_override in overrides.package_overrides.clone() {
                    let value = serde_json::json!({
                      path_override.path.clone(): path_override.value
                    });

                    let mut overrides: PackageOverrides =
                        serde_json::from_value(value)?;

                    if let Some(existing) =
                        map.get(&path_override.package_name).cloned()
                    {
                        overrides.merge(existing);
                    }

                    map.insert(path_override.package_name.clone(), overrides);
                }

                Ok(())
            };

        match &self.command {
            Command::ReleasePR { overrides, .. } => {
                map_overrides(overrides)?;
            }
            Command::StartNext { overrides, .. } => {
                map_overrides(overrides)?;
            }
            Command::Get {
                command: GetCommand::NextRelease { overrides, .. },
            } => {
                map_overrides(overrides)?;
            }
            _ => {}
        };

        Ok(map)
    }

    pub fn get_global_overrides(&self) -> GlobalOverrides {
        let mut global_overrides = GlobalOverrides {
            base_branch: self.base_branch.clone(),
            ..GlobalOverrides::default()
        };

        let cmd_overrides = match &self.command {
            Command::ReleasePR { overrides, .. } => Some(overrides),
            Command::StartNext { overrides, .. } => Some(overrides),
            Command::Get {
                command: GetCommand::NextRelease { overrides, .. },
            } => Some(overrides),
            _ => None,
        };

        if let Some(overrides) = cmd_overrides {
            global_overrides.tag_prefix = overrides.tag_prefix.clone();
            global_overrides.prerelease_suffix =
                overrides.prerelease_suffix.clone();
            global_overrides.prerelease_strategy =
                overrides.prerelease_strategy;
        }

        global_overrides
    }
}

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

fn get_remote_config(
    forge: ForgeType,
    repo: &str,
    token: Option<String>,
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
        return Err(ReleasaurusError::AuthenticationError(
            "Token not provided".to_string(),
        ));
    }

    let host = parsed.host.ok_or_else(|| -> ReleasaurusError {
        ReleasaurusError::InvalidRemoteUrl(
            "unable to parse host from repo".to_string(),
        )
    })?;

    let owner = parsed.owner.ok_or_else(|| -> ReleasaurusError {
        ReleasaurusError::InvalidRemoteUrl(
            "unable to parse owner from repo".to_string(),
        )
    })?;

    let project_path = parsed
        .path
        .strip_prefix("/")
        .wrap_err("failed to process project path")?
        .to_string();

    let link_base_url = format!("{}://{}", parsed.scheme, host);

    let release_link_base_url = match forge {
        ForgeType::Github => {
            format!("{}/{}/{}/releases/tag", link_base_url, owner, parsed.name)
        }
        ForgeType::Gitlab => {
            format!("{}/{}/-/releases", link_base_url, project_path)
        }
        ForgeType::Gitea => {
            format!("{}/{}/{}/releases", link_base_url, owner, parsed.name)
        }
        ForgeType::Local => "".into(),
    };

    let compare_link_base_url = match forge {
        ForgeType::Github => {
            format!("{}/{}/{}/compare", link_base_url, owner, parsed.name)
        }
        ForgeType::Gitlab => {
            format!("{}/{}/-/compare", link_base_url, project_path)
        }
        ForgeType::Gitea => {
            format!("{}/{}/{}/compare", link_base_url, owner, parsed.name)
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
        compare_link_base_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gets_github_remote() {
        let repo = "https://github.com/github_owner/github_repo".to_string();
        let token = "github_token".to_string();

        let forge_args = ForgeArgs {
            forge: Some(ForgeType::Github),
            repo: Some(repo),
            token: Some(token),
        };

        let remote = forge_args.get_remote().unwrap();

        assert!(matches!(remote, Remote::Github(_)));
    }

    #[test]
    fn gets_gitlab_remote() {
        let repo = "https://gitlab.com/gitlab_owner/gitlab_repo".to_string();
        let token = "gitlab_token".to_string();

        let forge_args = ForgeArgs {
            forge: Some(ForgeType::Gitlab),
            repo: Some(repo),
            token: Some(token),
        };

        let remote = forge_args.get_remote().unwrap();

        assert!(matches!(remote, Remote::Gitlab(_)));
    }

    #[test]
    fn gets_gitea_remote() {
        let repo = "http://gitea.com/gitea_owner/gitea_repo".to_string();
        let token = "gitea_token".to_string();

        let forge_args = ForgeArgs {
            forge: Some(ForgeType::Gitea),
            repo: Some(repo),
            token: Some(token),
        };

        let remote = forge_args.get_remote().unwrap();

        assert!(matches!(remote, Remote::Gitea(_)));
    }

    #[test]
    fn gets_local_remote() {
        let repo = ".".to_string();

        let forge_args = ForgeArgs {
            forge: Some(ForgeType::Local),
            repo: Some(repo),
            token: None,
        };

        let remote = forge_args.get_remote().unwrap();

        assert!(matches!(remote, Remote::Local(_)));
    }

    #[test]
    fn only_supports_http_and_https_schemes() {
        let repo = "git@gitea.com:gitea_owner/gitea_repo".to_string();
        let token = "gitea_token".to_string();

        let forge_args = ForgeArgs {
            forge: Some(ForgeType::Gitea),
            repo: Some(repo),
            token: Some(token),
        };

        let result = forge_args.get_remote();
        assert!(result.is_err());
    }

    #[test]
    fn validate_sha_accepts_valid_short_sha() {
        validate_sha("abc123d").unwrap();
    }

    #[test]
    fn validate_sha_accepts_valid_full_sha() {
        validate_sha("abc123def456789012345678901234567890abcd").unwrap();
    }

    #[test]
    fn validate_sha_rejects_too_short() {
        let result = validate_sha("abc123");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReleasaurusError::InvalidConfig(_)
        ));
    }

    #[test]
    fn validate_sha_rejects_too_long() {
        let result = validate_sha("abc123def456789012345678901234567890abcdef");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReleasaurusError::InvalidConfig(_)
        ));
    }

    #[test]
    fn validate_sha_rejects_non_hex_characters() {
        let result = validate_sha("abc123g");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReleasaurusError::InvalidConfig(_)
        ));
    }

    #[test]
    fn validate_sha_accepts_uppercase_hex() {
        validate_sha("ABC123D").unwrap();
    }

    #[test]
    fn validate_sha_trims_whitespace() {
        validate_sha("  abc123d  ").unwrap();
    }

    #[test]
    fn parse_reworded_commit_succeeds_with_valid_sha() {
        let reworded =
            parse_reworded_commit("abc123d=fix: new message").unwrap();
        assert_eq!(reworded.sha, "abc123d");
        assert_eq!(reworded.message, "fix: new message");
    }

    #[test]
    fn parse_reworded_commit_fails_with_invalid_sha() {
        let result = parse_reworded_commit("abc=fix: new message");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReleasaurusError::InvalidConfig(_)
        ));
    }

    #[test]
    fn parse_reworded_commit_fails_with_missing_equals() {
        let result = parse_reworded_commit("abc123dfix: new message");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReleasaurusError::InvalidConfig(_)
        ));
    }

    #[test]
    fn parse_reworded_commit_handles_multiline_message() {
        let reworded =
            parse_reworded_commit("abc123d=fix: new message\n\nMore content")
                .unwrap();
        assert_eq!(reworded.message, "fix: new message\n\nMore content");
    }

    #[test]
    fn parse_reworded_commit_trims_sha_whitespace() {
        let reworded =
            parse_reworded_commit("  abc123d  =fix: new message").unwrap();
        assert_eq!(reworded.sha, "abc123d");
    }
}
