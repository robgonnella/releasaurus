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
//! - Tagging and Publishing releases to various forge platforms
//!   (GitHub, GitLab, Gitea)
//!
//! See complete documentation at <https://releasaurus.rgon.io>

use clap::Parser;
use color_eyre::eyre::{Result, bail};
use releasaurus::cli::{Cli, Command, GetCommand, get};
use releasaurus_core::config::overrides::PackageOverrides;
use releasaurus_core::forge::manager::{ForgeManager, ForgeOptions};
use releasaurus_core::orchestrator::Orchestrator;
use releasaurus_core::resolver::Resolver;
use std::collections::HashMap;
use std::io::{BufRead, IsTerminal, Write};
use std::rc::Rc;

const DEBUG_ENV_VAR: &str = "RELEASAURUS_DEBUG";
const DRY_RUN_ENV_VAR: &str = "RELEASAURUS_DRY_RUN";

fn silence_logs(cli: &Cli) -> bool {
    let mut silent = false;

    if let Command::Get { command, .. } = &cli.command {
        match command {
            GetCommand::NextRelease { out_file, .. } => {
                if out_file.is_none() {
                    silent = true;
                }
            }
            GetCommand::CurrentRelease { out_file, .. } => {
                if out_file.is_none() {
                    silent = true;
                }
            }
            GetCommand::Release { out_file, .. } => {
                if out_file.is_none() {
                    silent = true;
                }
            }
            GetCommand::RecompiledNotes { out_file, .. } => {
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
        Command::Release { dry_run, .. } => dry_run,
        Command::ReleasePR { dry_run, .. } => dry_run,
        Command::StartNext { dry_run, .. } => dry_run,
        Command::ReleaseDirect { dry_run, .. } => dry_run,
        Command::Get { .. } => false,
    }
}

async fn create_orchestrator(
    cli: &Cli,
    dry_run: bool,
) -> Result<(Orchestrator, String)> {
    let mut forge = cli.forge_args.forge().await?;

    let global_overrides = cli.get_global_overrides();
    let package_overrides = cli.get_package_overrides()?;
    let commit_modifiers = cli.get_commit_modifiers();

    let config = Rc::new(
        forge
            .load_config(
                global_overrides.base_branch.clone(),
                cli.config
                    .as_deref()
                    .map(|p| p.to_string_lossy().into_owned()),
            )
            .await?,
    );

    forge.set_commit_search_depth(config.repository.first_release_search_depth);
    forge.set_tag_search_depth(config.repository.tag_search_depth);

    let forge_manager = ForgeManager::new(forge, ForgeOptions { dry_run });

    log::debug!("cli global overrides: {:#?}", global_overrides);
    log::debug!("cli package overrides: {:#?}", package_overrides);
    log::debug!("cli commit modifiers: {:#?}", commit_modifiers);

    let repo_name = forge_manager.repo_name();
    let default_branch = forge_manager.default_branch().to_string();
    let release_link_base_url = forge_manager.release_link_base_url();
    let compare_link_base_url = forge_manager.compare_link_base_url();

    let resolver = Resolver::builder()
        .commit_modifiers(commit_modifiers)
        .global_overrides(global_overrides)
        .package_overrides(
            package_overrides
                .into_iter()
                .map(|(k, v)| (k, PackageOverrides::from(v)))
                .collect::<HashMap<_, _>>(),
        )
        .release_link_base_url(release_link_base_url.clone())
        .compare_link_base_url(compare_link_base_url.clone())
        .repo_default_branch(default_branch.clone())
        .repo_name(repo_name)
        .toml_config(Rc::clone(&config))
        .build()?;

    let resolved_config = resolver.resolve(config.packages.clone())?;

    let orchestrator = Orchestrator::builder()
        .config(resolved_config)
        .forge(Rc::new(forge_manager))
        .build()?;

    Ok((orchestrator, default_branch))
}

/// Blocks until the user acknowledges what `release-direct` is about to
/// do. Unlike the PR flow there is nothing to review afterwards, and a
/// run that fails part way through cannot be finished by re-running it,
/// so the acknowledgement has to come first.
///
/// Generic over its streams so it can be exercised without a terminal.
fn confirm_release_direct<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    repo: &str,
    base_branch: &str,
) -> Result<()> {
    writeln!(
        writer,
        "release-direct will make changes that re-running it cannot undo.\n\
         \n  \
         repository: {repo}\n  \
         branch:     {base_branch}\n\
         \n\
         It commits the version bumps and changelog to that branch, creates \
         and pushes the release tag(s), and publishes the release(s) on your \
         forge. No pull request is created and there is no review step.\n"
    )?;

    write!(writer, "Type 'yes' to continue: ")?;
    writer.flush()?;

    let mut answer = String::new();
    reader.read_line(&mut answer)?;

    if !answer.trim().eq_ignore_ascii_case("yes") {
        bail!("aborted: release-direct was not confirmed");
    }

    Ok(())
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

    let (orchestrator, default_branch) =
        create_orchestrator(&cli, dry_run).await?;

    // Captured before the match takes `cli.command` by value.
    let repo = cli.forge_args.repo.clone();
    let base_branch = cli.base_branch.clone();

    // wrap all errors using ? and manually return Ok(()) to get the benefit
    // of eyre Report
    match cli.command {
        Command::ReleasePR { package, .. } => {
            orchestrator.create_release_prs(package).await?;
            Ok(())
        }
        Command::Release { package, .. } => {
            orchestrator.create_releases(package).await?;
            Ok(())
        }
        Command::Get { command } => {
            get::execute(orchestrator, command).await?;
            Ok(())
        }
        Command::StartNext { packages, .. } => {
            orchestrator.start_next_release(packages).await?;
            Ok(())
        }
        Command::ReleaseDirect {
            package,
            auto_approve,
            ..
        } => {
            // Nothing is written under dry_run, so there is nothing to
            // acknowledge and prompting would break it in CI.
            if !auto_approve && !dry_run {
                if !std::io::stdin().is_terminal() {
                    bail!(
                        "release-direct needs confirmation but stdin is not a \
                         terminal: pass --auto-approve to run \
                         non-interactively"
                    );
                }

                confirm_release_direct(
                    &mut std::io::stdin().lock(),
                    &mut std::io::stdout(),
                    repo.as_deref().unwrap_or("<unknown>"),
                    base_branch.as_deref().unwrap_or(&default_branch),
                )?;
            }

            orchestrator.release_direct(package).await?;
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
    fn silence_logs_returns_true_for_get_next_release_without_out_file() {
        let args = [
            create_base_args(),
            vec!["get".to_string(), "next-release".to_string()],
        ]
        .concat();
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(silence_logs(&cli));
    }

    #[test]
    fn silence_logs_returns_false_for_get_next_release_with_out_file() {
        let args = [
            create_base_args(),
            vec![
                "get".to_string(),
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
    fn silence_logs_returns_true_for_get_current_release_without_out_file() {
        let args = [
            create_base_args(),
            vec!["get".to_string(), "current-release".to_string()],
        ]
        .concat();
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(silence_logs(&cli));
    }

    #[test]
    fn silence_logs_returns_false_for_get_current_release_with_out_file() {
        let args = [
            create_base_args(),
            vec![
                "get".to_string(),
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
    fn silence_logs_returns_true_for_get_release_without_out_file() {
        let args = [
            create_base_args(),
            vec![
                "get".to_string(),
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
    fn silence_logs_returns_false_for_get_release_with_out_file() {
        let args = [
            create_base_args(),
            vec![
                "get".to_string(),
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
    fn silence_logs_returns_true_for_get_notes_without_out_file() {
        let args = [
            create_base_args(),
            vec![
                "get".to_string(),
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
    fn silence_logs_returns_false_for_get_notes_with_out_file() {
        let args = [
            create_base_args(),
            vec![
                "get".to_string(),
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
    fn silence_logs_returns_false_for_non_get_commands() {
        let test_cases =
            vec!["release-pr", "release", "start-next", "release-direct"];

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

    /// Drives `confirm_release_direct` over in-memory streams and
    /// returns whether it accepted, plus what it printed.
    fn confirm(input: &str, base_branch: &str) -> (bool, String) {
        let mut reader = std::io::Cursor::new(input.as_bytes());
        let mut writer: Vec<u8> = vec![];

        let result = confirm_release_direct(
            &mut reader,
            &mut writer,
            "https://github.com/owner/repo",
            base_branch,
        );

        (result.is_ok(), String::from_utf8(writer).unwrap())
    }

    #[test]
    fn confirm_release_direct_accepts_yes_in_any_case_or_padding() {
        for input in ["yes\n", "YES\n", "Yes\n", "  yes  \n"] {
            let (accepted, _) = confirm(input, "main");
            assert!(accepted, "should have accepted {input:?}");
        }
    }

    /// A bare `y` is deliberately not enough: the whole point is that
    /// the acknowledgement is hard to type by reflex.
    #[test]
    fn confirm_release_direct_rejects_anything_else() {
        for input in ["y\n", "no\n", "\n", "yes please\n", ""] {
            let (accepted, _) = confirm(input, "main");
            assert!(!accepted, "should have rejected {input:?}");
        }
    }

    #[test]
    fn confirm_release_direct_names_the_branch_when_one_was_given() {
        let (_, prompt) = confirm("yes\n", "release/2.x");

        assert!(prompt.contains("release/2.x"), "prompt was: {prompt}");
        assert!(prompt.contains("https://github.com/owner/repo"));
    }

    #[test]
    fn release_direct_defaults_to_requiring_confirmation() {
        let args =
            [create_base_args(), vec!["release-direct".to_string()]].concat();
        let cli = Cli::try_parse_from(args).unwrap();

        let Command::ReleaseDirect { auto_approve, .. } = cli.command else {
            panic!("expected a release-direct command");
        };

        assert!(!auto_approve);
    }

    #[test]
    fn release_direct_accepts_auto_approve() {
        let args = [
            create_base_args(),
            vec!["release-direct".to_string(), "--auto-approve".to_string()],
        ]
        .concat();
        let cli = Cli::try_parse_from(args).unwrap();

        let Command::ReleaseDirect { auto_approve, .. } = cli.command else {
            panic!("expected a release-direct command");
        };

        assert!(auto_approve);
    }
}
