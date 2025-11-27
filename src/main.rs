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
//! ## Supported Languages & Frameworks
//!
//! - **Generic**: No Updates
//! - **Java**: Maven pom.xml, Gradle build files
//! - **Node.js**: package.json, package-lock.json, yarn.lock
//! - **PHP**: composer.json
//! - **Python**: pyproject.toml, setup.py, setup.cfg
//! - **Ruby**: gemspec files, version.rb files
//! - **Rust**: Cargo.toml and Cargo.lock version management
//!
//! ## Commands
//!
//! - `release-pr`: Create a release preparation pull request
//! - `release`: Execute the final release process
//!
//! ## Usage
//!
//! ```bash
//! releasaurus release-pr    # Create a release PR
//! releasaurus release       # Publish the release
//! ```

use clap::Parser;

use releasaurus::{cli, command, result::Result};

const DEBUG_ENV_VAR: &str = "RELEASAURUS_DEBUG";
const DRY_RUN_ENV_VAR: &str = "RELEASAURUS_DRY_RUN";

/// Initialize terminal logger with debug or info level filtering for
/// releasaurus output.
fn initialize_logger(debug: bool) -> Result<()> {
    let filter = if debug {
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

    let mut args = cli::Args::parse();

    if std::env::var(DEBUG_ENV_VAR).is_ok() {
        args.debug = true;
    }

    if std::env::var(DRY_RUN_ENV_VAR).is_ok() {
        args.dry_run = true;
    }

    if args.dry_run {
        args.debug = true;
    }

    initialize_logger(args.debug)?;

    let remote = args.get_remote()?;
    let forge = remote.get_forge().await?;

    match args.command {
        cli::Command::ReleasePR => command::release_pr::execute(forge).await,
        cli::Command::Release => command::release::execute(forge).await,
    }
}
