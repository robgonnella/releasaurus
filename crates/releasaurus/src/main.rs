use clap::Parser;
use color_eyre::eyre::Result;
use log::*;
use releasaurus_core::changelog::{
    config::{ChangelogConfig, PackageConfig},
    git_cliff::GitCliffChangelog,
    traits::Writer,
};
use std::{env, fs};

mod args;
mod config;

const DEFAULT_CONFIG_FILE: &str = "releasaurus.toml";

fn initialize_logger(debug: bool) {
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
    )
    .unwrap();
}

fn load_config() -> Result<config::CliConfig> {
    // search for config file walking up ancestors as necessary
    let maybe_found_config = env::current_dir()?.ancestors().find_map(|dir| {
        let path = dir.join(DEFAULT_CONFIG_FILE);
        if path.is_file() {
            info!("found config file: {}", path.display());
            return Some(path);
        }

        None
    });

    // process and use config file if found
    if let Some(config_file) = maybe_found_config {
        if let Some(dir) = config_file.parent() {
            // make sure to switch to directory of config file
            // so any paths defined in config work
            env::set_current_dir(dir)?;
        }

        if let Ok(content) = fs::read_to_string(config_file) {
            let cli_config: config::CliConfig = toml::from_str(&content)?;
            return Ok(cli_config);
        }
    }

    // otherwise return default config
    Ok(config::CliConfig::default())
}

fn main() -> Result<()> {
    let cli_args = args::Cli::parse();

    initialize_logger(cli_args.debug);

    let cli_config = load_config()?;

    let remote = cli_args.get_remote()?;

    for single in cli_config {
        let name = single.package.name.clone();
        let changelog = GitCliffChangelog::new(ChangelogConfig {
            body: single.changelog.body.clone(),
            header: single.changelog.header.clone(),
            footer: single.changelog.footer.clone(),
            package: PackageConfig {
                name: single.package.name.clone(),
                path: single.package.path.clone(),
                tag_prefix: single.package.tag_prefix.clone(),
            },
            remote: remote.clone(),
        })?;
        let output = changelog.write()?;

        info!("=============={}==============", name);
        println!("current_version: {:#?}", output.current_version);
        println!("next_version: {:#?}", output.next_version);
        println!("is_breaking: {}\n\n", output.is_breaking);
    }

    Ok(())
}
