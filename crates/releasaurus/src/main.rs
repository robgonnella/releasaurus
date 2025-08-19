use color_eyre::eyre::Result;
use log::*;
use releasaurus_core::{
    changelog::{git_cliff::GitCliffChangelog, traits::Writer},
    config::Config,
};
use std::fs;

fn initialize_logger(debug: bool) {
    let filter = if debug {
        simplelog::LevelFilter::Trace
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
    initialize_logger(true);

    let config = load_config()?;

    for package_config in config.into_iter() {
        let name = package_config.package.name.clone();
        let changelog = GitCliffChangelog::new(package_config)?;
        let output = changelog.write()?;

        info!("=============={}==============", name);
        println!("current_version: {:#?}", output.current_version);
        println!("next_version: {:#?}", output.next_version);
        println!("is_breaking: {}\n\n", output.is_breaking);
    }

    Ok(())
}
