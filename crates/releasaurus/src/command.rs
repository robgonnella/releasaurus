use std::{collections::HashMap, env};

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
use tempfile::TempDir;

use crate::{cli, config};

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfo {
    pub current_version: Option<String>,
    pub next_version: Option<String>,
}

pub fn release_pr(args: &cli::Args) -> Result<()> {
    let remote = args.get_remote()?;
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
    let cli_config = config::load_config()?;

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
