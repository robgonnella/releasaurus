//! Common functionality shared between release commands

use color_eyre::eyre::Result;
use log::*;
use std::{env, path::Path};
use tempfile::TempDir;

use crate::{
    analyzer::config::AnalyzerConfig,
    config,
    forge::{config::RemoteConfig, traits::Forge, types::PrLabelsRequest},
    repo::{Repository, StartingPoint},
};

/// Sets up a temporary repository for command execution
///
/// This function:
/// - Creates a temporary directory
/// - Clones the repository to the temp directory
/// - Changes the current working directory to the temp directory
///
/// Returns both the Repository instance and the TempDir handle.
/// The TempDir must be kept alive to prevent premature cleanup.
pub fn setup_repository(forge: &dyn Forge) -> Result<(Repository, TempDir)> {
    let remote_config = forge.config();
    let tmp_dir = TempDir::new()?;

    info!(
        "cloning repository {} to {}",
        remote_config.repo,
        tmp_dir.path().display()
    );

    let repo = Repository::new(tmp_dir.path(), remote_config.clone())?;

    info!(
        "switching directory to cloned repository: {}",
        tmp_dir.path().display(),
    );

    env::set_current_dir(tmp_dir.path())?;

    Ok((repo, tmp_dir))
}

/// Loads the CLI configuration from the current directory
///
/// This is a simple wrapper around config::load_config that adds
/// appropriate logging.
pub fn load_configuration() -> Result<config::CliConfig> {
    info!("loading configuration");
    config::load_config(None)
}

/// Creates a changelog configuration for a specific package
///
/// This helper consolidates the common pattern of creating ChangelogConfig
/// instances used by both release and release-pr commands.
pub fn create_changelog_config(
    package: &config::CliPackageConfig,
    cli_config: &config::CliConfig,
    remote_config: &RemoteConfig,
    starting_point: Option<StartingPoint>,
) -> AnalyzerConfig {
    AnalyzerConfig {
        body: cli_config.changelog.body.clone(),
        commit_link_base_url: remote_config.commit_link_base_url.clone(),
        footer: cli_config.changelog.footer.clone(),
        header: cli_config.changelog.header.clone(),
        package_path: package.path.clone(),
        release_link_base_url: remote_config.release_link_base_url.clone(),
        starting_point,
        tag_prefix: Some(get_tag_prefix(package)),
    }
}

/// Gets the tag prefix for a package, using "v" as default if none specified
pub fn get_tag_prefix(package: &config::CliPackageConfig) -> String {
    let mut default_for_package = "v".to_string();
    let package_path = Path::new(&package.path);
    if let Some(basename) = package_path.file_name() {
        default_for_package = format!("{}-v", basename.display());
    }
    package.tag_prefix.clone().unwrap_or(default_for_package)
}

/// Logs package processing information
pub fn log_package_processing(package_path: &str, tag_prefix: &str) {
    info!(
        "processing changelog for package path: {}, tag_prefix: {}",
        package_path, tag_prefix
    );
}

/// Sets up a release branch for PR creation
///
/// Creates a branch name using the default PR prefix and the repository's
/// default branch, then creates and switches to that branch.
pub fn setup_release_branch(
    repo: &Repository,
    pr_branch_prefix: &str,
) -> Result<String> {
    let release_branch = format!("{}{}", pr_branch_prefix, repo.default_branch);

    debug!("setting up release branch: {release_branch}");
    repo.create_branch(&release_branch)?;
    repo.switch_branch(&release_branch)?;

    Ok(release_branch)
}

/// Updates PR labels using the forge API
///
/// This is a common pattern used by both release commands to set
/// specific labels on pull requests.
pub fn update_pr_labels(
    forge: &dyn Forge,
    pr_number: u64,
    labels: Vec<String>,
) -> Result<()> {
    let req = PrLabelsRequest { pr_number, labels };

    forge.replace_pr_labels(req)
}

/// Commits and pushes changes to a branch
///
/// This is a common pattern for both release commands that need to
/// commit changes and push them to the remote repository.
pub fn commit_and_push_changes(
    repo: &Repository,
    commit_message: &str,
    branch: &str,
) -> Result<()> {
    repo.add_all()?;
    repo.commit(commit_message)?;
    repo.push_branch(branch)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{CliChangelogConfig, CliPackageConfig},
        repo::StartingPoint,
    };

    #[test]
    fn test_get_tag_prefix_with_prefix() {
        let package = CliPackageConfig {
            path: "test".to_string(),
            tag_prefix: Some("release-".to_string()),
        };

        assert_eq!(get_tag_prefix(&package), "release-");
    }

    #[test]
    fn test_get_tag_prefix_default() {
        let package = CliPackageConfig {
            path: "test".to_string(),
            tag_prefix: None,
        };

        assert_eq!(get_tag_prefix(&package), "test-v");
    }

    #[test]
    fn test_get_tag_prefix_root_directory() {
        let package = CliPackageConfig {
            path: ".".to_string(),
            tag_prefix: None,
        };

        assert_eq!(get_tag_prefix(&package), "v");
    }

    #[test]
    fn test_create_changelog_config() {
        let package = CliPackageConfig {
            path: "./test".to_string(),
            tag_prefix: Some("v".to_string()),
        };

        let cli_config = config::CliConfig {
            changelog: CliChangelogConfig {
                body: "test body".to_string(),
                header: Some("test header".to_string()),
                footer: Some("test footer".to_string()),
            },
            packages: vec![],
        };

        let remote_config = RemoteConfig {
            host: "example.com".to_string(),
            scheme: "https".to_string(),
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            path: "path".to_string(),
            commit_link_base_url: "https://example.com/commit".to_string(),
            release_link_base_url: "https://example.com/releases".to_string(),
            token: "token".into(),
        };

        let changelog_config = create_changelog_config(
            &package,
            &cli_config,
            &remote_config,
            Some(StartingPoint {
                tagged_commit: "abc123".into(),
                tagged_parent: "def123".into(),
            }),
        );

        assert_eq!(changelog_config.package_path, "./test");
        assert_eq!(changelog_config.body, "test body");
        assert_eq!(changelog_config.header, Some("test header".to_string()));
        assert_eq!(changelog_config.footer, Some("test footer".to_string()));
        assert_eq!(
            changelog_config.starting_point,
            Some(StartingPoint {
                tagged_commit: "abc123".into(),
                tagged_parent: "def123".into(),
            })
        );
        assert_eq!(
            changelog_config.commit_link_base_url,
            "https://example.com/commit"
        );
        assert_eq!(
            changelog_config.release_link_base_url,
            "https://example.com/releases"
        );
    }
}
