use color_eyre::eyre::Result;
use log::*;
use releasaurus_core::{
    changelog::{git_cliff::GitCliffChangelog, traits::Writer},
    config::Config,
};
use std::{env, fs};

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

fn load_config() -> Result<Config> {
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
            let config: Config = toml::from_str(&content)?;
            return Ok(config);
        }
    }

    // otherwise return default config
    Ok(Config::default())
}

fn main() -> Result<()> {
    initialize_logger(false);

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
