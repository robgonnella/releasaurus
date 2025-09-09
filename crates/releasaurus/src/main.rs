use color_eyre::eyre::Result;
use log::*;
use releasaurus_core::{
    changelog::{
        git_cliff::GitCliffChangelog,
        traits::{CurrentVersion, Generator, NextVersion},
    },
    config::{Config, SinglePackageConfig},
};
use std::fs;

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

fn load_config() -> Result<Config> {
    if let Ok(content) = fs::read_to_string("releasaurus.toml") {
        let config: Config = toml::from_str(&content)?;
        return Ok(config);
    }

    Ok(Config::default())
}

fn main() -> Result<()> {
    initialize_logger(false);

    let config = load_config()?;

    for entry in config.packages.iter().enumerate() {
        let single_package_config =
            SinglePackageConfig::from_config_index(config.clone(), entry.0);
        let changelog = GitCliffChangelog::new(single_package_config.clone())?;
        let output = changelog.generate()?;
        let current_version = changelog.current_version();
        let next_version = changelog.next_version();
        let is_breaking = changelog.next_is_breaking()?;

        info!(
            "=============={}==============",
            single_package_config.package.name
        );
        println!("{output}");
        println!("current_version: {:#?}", current_version);
        println!("next_version: {:#?}", next_version);
        println!("is_breaking: {}\n\n", is_breaking);
    }

    Ok(())
}
