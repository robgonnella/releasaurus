use clap::Parser;
use color_eyre::eyre::Result;

mod cli;
mod command;
mod config;

fn initialize_logger(debug: bool) -> Result<()> {
    let filter = if debug {
        simplelog::LevelFilter::Debug
    } else {
        simplelog::LevelFilter::Info
    };

    simplelog::TermLogger::init(
        filter,
        simplelog::Config::default(),
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
        cli::Command::ReleasePR => command::release_pr(&cli_args),
    }
}
