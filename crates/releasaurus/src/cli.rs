//! CLI top-level definition for release automation workflow.

use clap::{Args, Parser, Subcommand, ValueEnum};
use git_url_parse::{GitUrl, types::provider::GenericProvider};
use merge::Merge;
use releasaurus_core::{
    config::{changelog::RewordedCommit, prerelease::PrereleaseStrategy},
    error::{ReleasaurusError, Result},
    forge::{
        config::{RepoUrl, Scheme, TokenVar, resolve_token},
        gitea::Gitea,
        github::Github,
        gitlab::Gitlab,
        local::{LocalRepo, Remote},
        traits::Forge,
    },
    orchestrator::config::{
        CommitModifiers, GlobalOverrides, PackageOverrides, validate_sha,
    },
};
use secrecy::SecretString;
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

pub mod get;

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

    /// Optional path to local repository. Performs local git operations for
    /// commit analysis, file updates, commits, tagging, pushing, and only uses
    /// remote forge for PR and release creation
    #[arg(long, global = true)]
    pub local_path: Option<PathBuf>,

    /// Authentication token. Falls back to env vars:
    /// GITHUB_TOKEN, GITLAB_TOKEN, GITEA_TOKEN
    #[arg(short, long, global = true)]
    pub token: Option<SecretString>,
}

impl ForgeArgs {
    pub async fn forge(&self) -> Result<Box<dyn Forge>> {
        if let Some(forge_type) = self.forge.as_ref()
            && let Some(git_url) = self.repo.as_ref()
        {
            let forge: Box<dyn Forge> = match forge_type {
                ForgeType::Github => {
                    let repo = git_url_to_repo_url(git_url)?;
                    let github =
                        Github::new(repo.clone(), self.token.clone()).await?;
                    if let Some(local_path) = self.local_path.as_ref() {
                        self.resolve_hybrid_forge(
                            Arc::new(github),
                            local_path,
                            &repo,
                            TokenVar::Github,
                        )?
                    } else {
                        Box::new(github)
                    }
                }
                ForgeType::Gitlab => {
                    let repo = git_url_to_repo_url(git_url)?;
                    let gitlab =
                        Gitlab::new(repo.clone(), self.token.clone()).await?;
                    if let Some(local_path) = self.local_path.as_ref() {
                        self.resolve_hybrid_forge(
                            Arc::new(gitlab),
                            local_path,
                            &repo,
                            TokenVar::Gitlab,
                        )?
                    } else {
                        Box::new(gitlab)
                    }
                }
                ForgeType::Gitea => {
                    let repo = git_url_to_repo_url(git_url)?;
                    let gitea =
                        Gitea::new(repo.clone(), self.token.clone()).await?;
                    if let Some(local_path) = self.local_path.as_ref() {
                        self.resolve_hybrid_forge(
                            Arc::new(gitea),
                            local_path,
                            &repo,
                            TokenVar::Gitea,
                        )?
                    } else {
                        Box::new(gitea)
                    }
                }
                ForgeType::Local => {
                    Box::new(LocalRepo::new(Path::new(git_url), None)?)
                }
            };

            Ok(forge)
        } else {
            Err(ReleasaurusError::InvalidArgs(
                "both --forge and --repo are required".into(),
            ))
        }
    }

    fn resolve_hybrid_forge(
        &self,
        forge: Arc<dyn Forge>,
        local_path: &Path,
        repo: &RepoUrl,
        token_var: TokenVar,
    ) -> Result<Box<dyn Forge>> {
        let token =
            resolve_token(self.token.clone(), repo.token.as_ref(), token_var)?;

        Ok(Box::new(LocalRepo::new(
            local_path,
            Some(Remote {
                forge,
                token,
                url: repo.clone(),
            }),
        )?))
    }
}

fn git_url_to_repo_url(url: &str) -> Result<RepoUrl> {
    let git_url = GitUrl::parse(url).map_err(|e| {
        ReleasaurusError::InvalidArgs(format!(
            "failed to parse repo url as git url: {}",
            e
        ))
    })?;

    let url_scheme = git_url.scheme().ok_or(ReleasaurusError::InvalidArgs(
        "failed to parse scheme from repo url".into(),
    ))?;

    if url_scheme != "https" && url_scheme != "http" {
        return Err(ReleasaurusError::InvalidArgs(
            "only https and http schemes are supported for repo urls".into(),
        ));
    }

    let scheme = if url_scheme == "https" {
        Scheme::Https
    } else {
        Scheme::Http
    };

    let provider: GenericProvider = git_url.provider_info().map_err(|e| {
        ReleasaurusError::InvalidArgs(format!(
            "failed to parse provider info from repo url: {}",
            e
        ))
    })?;

    let host = git_url.host().ok_or(ReleasaurusError::InvalidArgs(
        "failed to parse host from repo url".into(),
    ))?;

    let owner = provider.owner();
    let name = provider.repo();
    let path = git_url.path();
    let port = git_url.port();
    let token = git_url.password().map(SecretString::from);

    Ok(RepoUrl {
        host: host.to_string(),
        owner: owner.to_string(),
        name: name.to_string(),
        path: path.to_string(),
        port,
        scheme,
        token,
    })
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
pub struct CliPackageOverrides {
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

impl From<CliPackageOverrides> for PackageOverrides {
    fn from(value: CliPackageOverrides) -> Self {
        Self {
            prerelease_strategy: value.prerelease_strategy,
            prerelease_suffix: value.prerelease_suffix,
            tag_prefix: value.tag_prefix,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct SharedCommandOverrides {
    /// Override package properties using dot notation
    /// Example: --set-package my-pkg.prerelease.suffix=beta
    #[arg(
        long = "set-package",
        value_parser = parse_package_override,
        value_name = "KEY=VALUE"
    )]
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
    #[arg(long, value_parser = parse_prerelease_strategy)]
    prerelease_strategy: Option<PrereleaseStrategy>,
}

#[derive(Debug, Clone, Default, Args)]
pub struct CliCommitModifiers {
    /// Commit sha (or prefix) to skip when calculating next version and
    /// generating changelog. Matches any commit whose SHA starts with the
    /// provided value. Can be provided more than once to skip multiple commits
    #[arg(
        long = "skip-sha",
        value_parser = validate_sha,
        value_name = "SKIP_SHA"
    )]
    pub skip_shas: Vec<String>,

    /// Rewords a commit message when generating changelog. Must be in the
    /// form "sha=message". The SHA can be a prefix - matches any commit whose
    /// SHA starts with the provided value. Can be provided more than once to
    /// reword multiple commits
    /// Example: --reword "abc123de=fix: a new message\n\nMore content"
    #[arg(long, value_parser = parse_reworded_commit, value_name = "KEY=VALUE")]
    pub reword: Vec<RewordedCommit>,
}

impl From<CommitModifiers> for CliCommitModifiers {
    fn from(value: CommitModifiers) -> Self {
        Self {
            reword: value.reword,
            skip_shas: value.skip_shas,
        }
    }
}

impl From<CliCommitModifiers> for CommitModifiers {
    fn from(value: CliCommitModifiers) -> Self {
        Self {
            reword: value.reword,
            skip_shas: value.skip_shas,
        }
    }
}

fn parse_prerelease_strategy(s: &str) -> Result<PrereleaseStrategy> {
    s.parse::<PrereleaseStrategy>().map_err(|_| {
        ReleasaurusError::invalid_config(format!(
            "Invalid prerelease strategy: '{}'. \
             Valid values: versioned, static",
            s
        ))
    })
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
            "Invalid --reword format: '{}'. Expected \
             'commit_sha=new_message'. Example: \
             --reword 'abc123de=fix: corrected message'",
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
        commit_modifiers: CliCommitModifiers,

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
        commit_modifiers: CliCommitModifiers,

        #[command(flatten)]
        overrides: SharedCommandOverrides,

        /// Targets a specific package in config for release PR generation
        #[arg(short, long)]
        package: Option<String>,

        /// Execute in dry-run mode
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Create a git tag and publish release after PR merge
    Release {
        /// Targets a specific package in config for release generation
        #[arg(short, long)]
        package: Option<String>,

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
    pub fn get_commit_modifiers(&self) -> CliCommitModifiers {
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
            _ => CliCommitModifiers::default(),
        }
    }

    /// Gathers the list of provided path override options, like
    /// --releasaurus.prerelease.suffix=beta, and collects them into a single
    /// struct of all allowed overrides properties for each package name.
    /// Returns HashMap<pkg_name, Option<PackageOverrides>>
    pub fn get_package_overrides(
        &self,
    ) -> Result<HashMap<String, CliPackageOverrides>> {
        let mut map: HashMap<String, CliPackageOverrides> = HashMap::new();

        let mut map_overrides =
            |overrides: &SharedCommandOverrides| -> Result<()> {
                for path_override in overrides.package_overrides.clone() {
                    let value = serde_json::json!({
                      path_override.path.clone(): path_override.value
                    });

                    let mut overrides: CliPackageOverrides =
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn forge_args_errors_if_missing_forge_type() {
        let repo = "https://github.com/github_owner/github_repo";

        let token = SecretString::from("github_token");

        let forge_args = ForgeArgs {
            forge: None,
            repo: Some(repo.to_string()),
            token: Some(token),
            local_path: None,
        };

        let result = forge_args.forge().await;

        match result {
            Ok(_) => {
                unreachable!("missing forge type should have resulted in error")
            }
            Err(err) => {
                assert!(matches!(err, ReleasaurusError::InvalidArgs(_)))
            }
        }
    }

    #[tokio::test]
    async fn forge_args_errors_if_missing_repo() {
        let token = SecretString::from("github_token");

        let forge_args = ForgeArgs {
            forge: Some(ForgeType::Github),
            repo: None,
            token: Some(token),
            local_path: None,
        };

        let result = forge_args.forge().await;

        match result {
            Ok(_) => {
                unreachable!("missing repo should have resulted in error")
            }
            Err(err) => {
                assert!(matches!(err, ReleasaurusError::InvalidArgs(_)))
            }
        }
    }

    #[tokio::test]
    async fn forge_args_local_forge_accepts_local_path_as_repo() {
        use std::process::Command;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let path = dir.path();

        // Initialise a bare-minimum git repo so LocalRepo::new succeeds
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(path)
            .output()
            .unwrap();
        std::fs::write(path.join("README"), "init").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(path)
            .output()
            .unwrap();

        let forge_args = ForgeArgs {
            forge: Some(ForgeType::Local),
            repo: Some(path.to_string_lossy().to_string()),
            token: None,
            local_path: None,
        };

        forge_args.forge().await.unwrap();
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
