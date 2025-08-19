use color_eyre::eyre::Result;
use log::*;
use releasaurus_core::{
    changelog::{
        git_cliff::GitCliffChangelog,
        traits::{CurrentVersion, Generator, NextVersion},
    },
    config::Config,
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

    for package_config in config.into_iter() {
        let name = package_config.package.name.clone();
        let changelog = GitCliffChangelog::new(package_config)?;
        let output = changelog.generate()?;
        let current_version = changelog.current_version();
        let next_version = changelog.next_version();
        let is_breaking = changelog.next_is_breaking()?;

        info!("=============={}==============", name);
        println!("{output}");
        println!("current_version: {:#?}", current_version);
        println!("next_version: {:#?}", next_version);
        println!("is_breaking: {}\n\n", is_breaking);
    }

    Ok(())
}
