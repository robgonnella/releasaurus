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
//! ## Commands
//!
//! - `release-pr`: Create a release preparation pull request
//! - `release`: Execute the final release process
//! - `projected-release`: Outputs the entire projected next release object as json
//!
//! ## Usage
//!
//! ```bash
//! releasaurus release-pr        # Create a release PR
//! releasaurus release           # Publish the release
//! releasaurus projected-release # Output projected next release as json
//! ```

use clap::Parser;

use releasaurus::{
    Args, Command, Result, ShowCommand, release, release_pr, show,
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

    let mut args = Args::parse();

    let mut silence_logs = false;

    if std::env::var(DEBUG_ENV_VAR).is_ok() {
        args.debug = true;
    }

    if std::env::var(DRY_RUN_ENV_VAR).is_ok() {
        args.dry_run = true;
    }

    if args.dry_run {
        args.debug = true;
    }

    if let Command::Show { command } = &args.command {
        match command {
            ShowCommand::NextRelease { out_file, .. } => {
                if out_file.is_none() {
                    silence_logs = true;
                }
            }
            ShowCommand::ReleaseNotes { out_file, .. } => {
                if out_file.is_none() {
                    silence_logs = true;
                }
            }
        }
    }

    initialize_logger(args.debug, silence_logs)?;

    let remote = args.get_remote()?;
    let forge_manager = remote.get_forge_manager().await?;

    match args.command {
        Command::ReleasePR => release_pr::execute(&forge_manager).await,
        Command::Release => release::execute(&forge_manager).await,
        Command::Show { command } => {
            show::execute(&forge_manager, command).await
        }
    }
}
