//! # Releasaurus
//!
//! A comprehensive release automation tool that streamlines the software
//! release process across multiple programming languages and forge platforms.
//!
//! ## Overview
//!
//! Releasaurus automates the entire release workflow including:
//! - Version detection and bumping across different project types
//! - Changelog generation
//! - Creating release pull requests
//! - Tagging and Publishing releases to various forge platforms (GitHub, GitLab, Gitea)
//!
//! See complete documentation at <https://releasaurus.rgon.io>
//! ```

use clap::Parser;

use releasaurus::{
    Cli, Command, ForgeFactory, ReleasaurusError, Result, ShowCommand,
    config::resolver::ConfigResolverBuilder, release, release_pr, show,
    start_next,
};

const DEBUG_ENV_VAR: &str = "RELEASAURUS_DEBUG";
const DRY_RUN_ENV_VAR: &str = "RELEASAURUS_DRY_RUN";

/// Initialize terminal logger with debug or info level filtering for
/// releasaurus output.
fn initialize_logger(debug: bool, silent: bool) -> Result<()> {
    let filter = if silent {
        simplelog::LevelFilter::Off
    } else if debug {
        simplelog::LevelFilter::Debug
    } else {
        simplelog::LevelFilter::Info
    };

    let config = simplelog::ConfigBuilder::new()
        .add_filter_allow_str("releasaurus")
        .build();

    simplelog::TermLogger::init(
        filter,
        config,
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;

    Ok(())
}

/// Main entry point that initializes error handling, logging, and dispatches
/// commands.
#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let mut cli = Cli::parse();

    let mut silence_logs = false;

    if std::env::var(DEBUG_ENV_VAR).is_ok() {
        cli.debug = true;
    }

    if std::env::var(DRY_RUN_ENV_VAR).is_ok() {
        cli.dry_run = true;
    }

    if cli.dry_run {
        cli.debug = true;
    }

    if let Command::Show { command, .. } = &cli.command {
        match command {
            ShowCommand::NextRelease { out_file, .. } => {
                if out_file.is_none() {
                    silence_logs = true;
                }
            }
            ShowCommand::CurrentRelease { out_file, .. } => {
                if out_file.is_none() {
                    silence_logs = true;
                }
            }
            ShowCommand::Release { out_file, .. } => {
                if out_file.is_none() {
                    silence_logs = true;
                }
            }
        }
    }

    initialize_logger(cli.debug, silence_logs)?;

    let remote = cli.get_remote()?;
    let forge_manager = ForgeFactory::create(&remote).await?;

    let global_overrides = cli.get_global_overrides();
    let package_overrides = cli.get_package_overrides()?;
    let commit_modifiers = cli.get_commit_modifiers();

    log::debug!("global overrides: {:#?}", global_overrides);
    log::debug!("package overrides: {:#?}", package_overrides);
    log::debug!("commit modifiers: {:#?}", commit_modifiers);

    let config = forge_manager
        .load_config(global_overrides.base_branch.clone())
        .await?;

    let repo_name = forge_manager.repo_name();
    let default_branch = forge_manager.default_branch();
    let release_link_base_url = forge_manager.release_link_base_url();

    let config = ConfigResolverBuilder::default()
        .config(config)
        .repo_name(repo_name)
        .repo_default_branch(default_branch)
        .release_link_base_url(release_link_base_url)
        .package_overrides(package_overrides)
        .global_overrides(global_overrides)
        .commit_modifiers(commit_modifiers)
        .build()
        .map_err(|e| {
            ReleasaurusError::invalid_config(format!(
                "Failed to build config resolver: {}",
                e
            ))
        })?
        .resolve()?;

    match cli.command {
        Command::ReleasePR { .. } => {
            release_pr::execute(&forge_manager, config).await
        }
        Command::Release => release::execute(&forge_manager, config).await,
        Command::Show { command } => {
            show::execute(&forge_manager, command, config).await
        }
        Command::StartNext { packages, .. } => {
            start_next::execute(&forge_manager, packages, config).await
        }
    }
}
