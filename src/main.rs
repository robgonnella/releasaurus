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
mod repo;
mod result;
mod updater;

use crate::result::Result;

/// Initialize the application logger with appropriate filtering and formatting.
///
/// Sets up terminal logging with colored output and filters to show only releasaurus
/// log messages. The log level is determined by the debug flag.
///
/// # Arguments
///
/// * `debug` - If true, sets log level to Debug; otherwise uses Info level
///
/// # Returns
///
/// * `Result<()>` - Ok(()) on successful initialization, Err on failure
///
/// # Errors
///
/// Returns an error if the terminal logger cannot be initialized due to
/// terminal compatibility issues or other system constraints.
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

/// Application entry point.
///
/// Performs the following initialization and execution steps:
/// 1. Installs color-eyre for enhanced error reporting
/// 2. Parses command line arguments using clap
/// 3. Initializes logging with appropriate level
/// 4. Dispatches to the appropriate command handler
///
/// # Returns
///
/// * `Result<()>` - Ok(()) on successful execution, Err on any failure
///
/// # Errors
///
/// Returns an error if:
/// - Color-eyre installation fails
/// - Logger initialization fails
/// - Command execution encounters an error
fn main() -> Result<()> {
    color_eyre::install()?;

    let cli_args = cli::Args::parse();

    initialize_logger(cli_args.debug)?;

    match cli_args.command {
        cli::Command::ReleasePR => command::release_pr::execute(&cli_args),
        cli::Command::Release => command::release::execute(&cli_args),
    }
}
