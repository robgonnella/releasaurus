use clap::Parser;
use color_eyre::eyre::Result;
use log::*;
use releasaurus_core::{
    changelog::{
        config::{ChangelogConfig, PackageConfig},
        git_cliff::GitCliffChangelog,
        traits::Writer,
    },
    forge::config::DEFAULT_PR_BRANCH_PREFIX,
    git::Git,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs};
use tempfile::TempDir;

mod args;
mod config;

const DEFAULT_CONFIG_FILE: &str = "releasaurus.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfo {
    pub current_version: Option<String>,
    pub next_version: Option<String>,
}

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
    info!(
        "no configuration file found for {DEFAULT_CONFIG_FILE}: using default config"
    );
    Ok(config::CliConfig::default())
}

fn main() -> Result<()> {
    let cli_args = args::Cli::parse();

    initialize_logger(cli_args.debug);

    let remote = cli_args.get_remote()?;
    let forge = remote.get_forge()?;
    let remote_config = forge.config();
    let tmp_dir = TempDir::new()?;

    info!(
        "cloning repository {} to {}",
        remote_config.repo,
        tmp_dir.path().display()
    );
    let git = Git::new(tmp_dir.path(), remote_config.clone())?;

    info!(
        "switching directory to cloned repository: {}",
        tmp_dir.path().display(),
    );
    env::set_current_dir(tmp_dir.path())?;

    info!("loading configuration");
    let cli_config = load_config()?;

    let release_branch =
        format!("{}{}", DEFAULT_PR_BRANCH_PREFIX, git.default_branch);

    git.create_branch(&release_branch)?;
    git.switch_branch(&release_branch)?;

    let mut manifest: HashMap<String, VersionInfo> = HashMap::new();

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
            commit_link_base_url: remote_config.commit_link_base_url.clone(),
            release_link_base_url: remote_config.release_link_base_url.clone(),
        })?;
        let output = changelog.write()?;
        let version_info = VersionInfo {
            current_version: output.current_version,
            next_version: output.next_version,
        };
        if name.is_empty() {
            manifest.insert(single.package.path, version_info);
        } else {
            manifest.insert(name, version_info);
        }
    }

    info!("manifest: {:#?}", manifest);

    Ok(())
}
