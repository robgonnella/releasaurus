//! # Releasaurus
//!
//! A comprehensive release automation tool that streamlines the software release process
//! across multiple programming languages and forge platforms.
//!
//! ## Overview
//!
//! Releasaurus automates the entire release workflow including:
//! - Version detection and bumping across different project types
//! - Changelog generation using git-cliff
//! - Creating release pull requests
//! - Publishing releases to various forge platforms (GitHub, GitLab, Gitea)
//!
//! ## Supported Languages & Frameworks
//!
//! - **Rust**: Cargo.toml version management
//! - **Node.js**: package.json and package-lock.json
//! - **Python**: pyproject.toml, setup.py, requirements files
//! - **Java**: Maven pom.xml, Gradle build files
//! - **PHP**: composer.json
//! - **Ruby**: Gemfile, gemspec files
//! - **Generic**: Version file patterns
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

mod analyzer;
mod cli;
mod command;
mod config;
mod forge;
mod result;
mod updater;

use crate::result::Result;

/// Initialize terminal logger with colored output and level filtering.
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

#[tokio::main]
/// Main entry point. Initializes error handling, logging, and dispatches commands.
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = cli::Args::parse();

    initialize_logger(args.debug)?;

    let remote = args.get_remote()?;

    match args.command {
        cli::Command::ReleasePR => command::release_pr::execute(remote).await,
        cli::Command::Release => command::release::execute(remote).await,
    }
}
