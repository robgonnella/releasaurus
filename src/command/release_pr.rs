use color_eyre::eyre::Result;
use log::*;
use std::{collections::HashMap, env};
use tempfile::TempDir;

use crate::{
    changelog::{
        config::{ChangelogConfig, PackageConfig},
        git_cliff::GitCliffChangelog,
        traits::{Output, Writer},
    },
    cli, config,
    forge::{
        config::{DEFAULT_PR_BRANCH_PREFIX, PENDING_LABEL},
        types::{
            CreatePrRequest, GetPrRequest, PrLabelsRequest, UpdatePrRequest,
        },
    },
    git::Git,
};

pub fn execute(args: &cli::Args) -> Result<()> {
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

    let mut manifest: HashMap<String, Output> = HashMap::new();

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
        if name.is_empty() {
            manifest.insert(single.package.path, output);
        } else {
            manifest.insert(name, output);
        }
    }

    let head_branch =
        format!("{}{}", DEFAULT_PR_BRANCH_PREFIX, git.default_branch);
    let base_branch = git.default_branch.clone();
    let mut title = format!("chore({}): release", git.default_branch);
    let mut body = vec![];
    let mut releasable = false;

    for (name, info) in manifest {
        debug!(
            "package: {}, current_version: {:#?}, next_version: {:#?}",
            name, info.current_version, info.next_version
        );
        if let Some(version) = info.next_version {
            info!("{name} is releasable: next version: {version}");
            releasable = true;
            let drop_down = format!(
                "<details><summary>{}</summary><br>{}</details>",
                version, info.changelog
            );
            body.push(drop_down);
            title = format!("{} {}", title, version)
        }
    }

    info!("creating pr");
    info!("title: {title}");

    if args.dry_run {
        info!("dry-run: skipping remote update");
        return Ok(());
    }

    if releasable {
        git.add_all()?;
        git.commit(&title)?;
        git.push_branch(&release_branch)?;

        let req = GetPrRequest {
            head_branch: head_branch.clone(),
            base_branch: base_branch.clone(),
        };

        let result = forge.get_pr_number(req)?;

        let pr_number = match result {
            Some(pr) => {
                info!("updating pr {pr}");
                let req = UpdatePrRequest {
                    pr_number: pr,
                    body: body.join("\n"),
                };

                forge.update_pr(req)?;
                pr
            }
            None => {
                info!("creating pull request");
                let req = CreatePrRequest {
                    title,
                    body: body.join("\n\n"),
                    head_branch,
                    base_branch,
                };

                forge.create_pr(req)?
            }
        };

        info!("setting labels for pr {pr_number}");
        let req = PrLabelsRequest {
            pr_number,
            labels: vec![PENDING_LABEL.into()],
        };

        forge.replace_pr_labels(req)?;
    }

    Ok(())
}
