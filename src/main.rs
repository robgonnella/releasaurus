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
use color_eyre::eyre::Result;
use std::rc::Rc;

use releasaurus::{
    Cli, Command, ForgeFactory, ForgeOptions, Orchestrator, OrchestratorConfig,
    ResolvedPackage, ResolvedPackageHash, ShowCommand, show,
};

const DEBUG_ENV_VAR: &str = "RELEASAURUS_DEBUG";
const DRY_RUN_ENV_VAR: &str = "RELEASAURUS_DRY_RUN";

fn silence_logs(cli: &Cli) -> bool {
    let mut silent = false;

    if let Command::Show { command, .. } = &cli.command {
        match command {
            ShowCommand::NextRelease { out_file, .. } => {
                if out_file.is_none() {
                    silent = true;
                }
            }
            ShowCommand::CurrentRelease { out_file, .. } => {
                if out_file.is_none() {
                    silent = true;
                }
            }
            ShowCommand::Release { out_file, .. } => {
                if out_file.is_none() {
                    silent = true;
                }
            }
            ShowCommand::Notes { out_file, .. } => {
                if out_file.is_none() {
                    silent = true;
                }
            }
        }
    }

    silent
}

/// Initialize terminal logger with debug or info level filtering for
/// releasaurus output.
fn initialize_logger(cli: &Cli) -> Result<()> {
    let silent = silence_logs(cli);

    let filter = if silent {
        simplelog::LevelFilter::Off
    } else if cli.debug {
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

fn get_dry_run_value(cli: &Cli) -> bool {
    if std::env::var(DRY_RUN_ENV_VAR).is_ok() {
        return true;
    }

    match cli.command {
        Command::Release { dry_run } => dry_run,
        Command::ReleasePR { dry_run, .. } => dry_run,
        Command::StartNext { dry_run, .. } => dry_run,
        _ => false,
    }
}

async fn create_orchestrator(cli: &Cli, dry_run: bool) -> Result<Orchestrator> {
    let remote = cli.forge_args.get_remote()?;

    let forge_manager =
        ForgeFactory::create(&remote, ForgeOptions { dry_run }).await?;

    let global_overrides = cli.get_global_overrides();
    let package_overrides = cli.get_package_overrides()?;
    let commit_modifiers = cli.get_commit_modifiers();

    log::debug!("global overrides: {:#?}", global_overrides);
    log::debug!("package overrides: {:#?}", package_overrides);
    log::debug!("commit modifiers: {:#?}", commit_modifiers);

    let config = Rc::new(
        forge_manager
            .load_config(global_overrides.base_branch.clone())
            .await?,
    );

    let repo_name = forge_manager.repo_name();
    let default_branch = forge_manager.default_branch();
    let release_link_base_url = forge_manager.release_link_base_url();

    let orchestrator_config = Rc::new(
        OrchestratorConfig::builder()
            .commit_modifiers(commit_modifiers)
            .global_overrides(global_overrides)
            .package_overrides(package_overrides)
            .release_link_base_url(release_link_base_url)
            .repo_default_branch(default_branch)
            .repo_name(repo_name)
            .toml_config(Rc::clone(&config))
            .build()?,
    );

    let mut resolved = vec![];

    for package_config in config.packages.iter() {
        resolved.push(
            ResolvedPackage::builder()
                .orchestrator_config(Rc::clone(&orchestrator_config))
                .package_config(package_config.clone())
                .build()?,
        );
    }

    let resolved_hash = ResolvedPackageHash::new(resolved)?;

    let orchestrator = Orchestrator::builder()
        .config(Rc::clone(&orchestrator_config))
        .package_configs(Rc::new(resolved_hash))
        .forge(Rc::new(forge_manager))
        .build()?;

    Ok(orchestrator)
}

/// Main entry point that initializes error handling, logging, and dispatches
/// commands.
#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let mut cli = Cli::parse();

    if std::env::var(DEBUG_ENV_VAR).is_ok() {
        cli.debug = true;
    }

    let dry_run = get_dry_run_value(&cli);

    if dry_run {
        cli.debug = true;
    }

    initialize_logger(&cli)?;

    let orchestrator = create_orchestrator(&cli, dry_run).await?;

    // wrap all errors using ? and manually return Ok(()) to get the benefit
    // of eyre Report
    match cli.command {
        Command::ReleasePR { .. } => {
            orchestrator.create_release_prs().await?;
            Ok(())
        }
        Command::Release { .. } => {
            orchestrator.create_releases().await?;
            Ok(())
        }
        Command::Show { command } => {
            show::execute(orchestrator, command).await?;
            Ok(())
        }
        Command::StartNext { packages, .. } => {
            orchestrator.start_next_release(packages).await?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_base_args() -> Vec<String> {
        vec![
            "releasaurus".to_string(),
            "--repo".to_string(),
            "https://github.com/test/repo".to_string(),
        ]
    }

    #[test]
    fn silence_logs_returns_true_for_show_next_release_without_out_file() {
        let args = [
            create_base_args(),
            vec!["show".to_string(), "next-release".to_string()],
        ]
        .concat();
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(silence_logs(&cli));
    }

    #[test]
    fn silence_logs_returns_false_for_show_next_release_with_out_file() {
        let args = [
            create_base_args(),
            vec![
                "show".to_string(),
                "next-release".to_string(),
                "--out-file".to_string(),
                "output.json".to_string(),
            ],
        ]
        .concat();
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(!silence_logs(&cli));
    }

    #[test]
    fn silence_logs_returns_true_for_show_current_release_without_out_file() {
        let args = [
            create_base_args(),
            vec!["show".to_string(), "current-release".to_string()],
        ]
        .concat();
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(silence_logs(&cli));
    }

    #[test]
    fn silence_logs_returns_false_for_show_current_release_with_out_file() {
        let args = [
            create_base_args(),
            vec![
                "show".to_string(),
                "current-release".to_string(),
                "--out-file".to_string(),
                "output.json".to_string(),
            ],
        ]
        .concat();
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(!silence_logs(&cli));
    }

    #[test]
    fn silence_logs_returns_true_for_show_release_without_out_file() {
        let args = [
            create_base_args(),
            vec![
                "show".to_string(),
                "release".to_string(),
                "--tag".to_string(),
                "v1.0.0".to_string(),
            ],
        ]
        .concat();
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(silence_logs(&cli));
    }

    #[test]
    fn silence_logs_returns_false_for_show_release_with_out_file() {
        let args = [
            create_base_args(),
            vec![
                "show".to_string(),
                "release".to_string(),
                "--tag".to_string(),
                "v1.0.0".to_string(),
                "--out-file".to_string(),
                "output.json".to_string(),
            ],
        ]
        .concat();
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(!silence_logs(&cli));
    }

    #[test]
    fn silence_logs_returns_true_for_show_notes_without_out_file() {
        let args = [
            create_base_args(),
            vec![
                "show".to_string(),
                "notes".to_string(),
                "--file".to_string(),
                "releases.json".to_string(),
            ],
        ]
        .concat();
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(silence_logs(&cli));
    }

    #[test]
    fn silence_logs_returns_false_for_show_notes_with_out_file() {
        let args = [
            create_base_args(),
            vec![
                "show".to_string(),
                "notes".to_string(),
                "--file".to_string(),
                "releases.json".to_string(),
                "--out-file".to_string(),
                "output.json".to_string(),
            ],
        ]
        .concat();
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(!silence_logs(&cli));
    }

    #[test]
    fn silence_logs_returns_false_for_non_show_commands() {
        let test_cases = vec!["release-pr", "release", "start-next"];

        for cmd in test_cases {
            let args = [create_base_args(), vec![cmd.to_string()]].concat();
            let cli = Cli::try_parse_from(args).unwrap();

            assert!(
                !silence_logs(&cli),
                "silence_logs should return false for {} command",
                cmd
            );
        }
    }
}
