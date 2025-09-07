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

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli_args = cli::Args::parse();

    initialize_logger(cli_args.debug)?;

    match cli_args.command {
        cli::Command::ReleasePR => command::release_pr::execute(&cli_args),
        cli::Command::Release => command::release::execute(&cli_args),
    }
}
