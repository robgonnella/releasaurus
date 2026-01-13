use clap::Parser;
use color_eyre::eyre::{Result, eyre};
use nucleo_matcher::{
    Config, Matcher,
    pattern::{CaseMatching, Normalization, Pattern},
};
use std::cmp::Reverse;
use std::path::Path;
use tokio::fs;

use releasaurus::SerializableReleasablePackage;

use crate::json_scripts::{
    PlatformClient,
    cli::{Cli, Command, Platform},
    logging::initialize_logger,
    slack::client::SlackClient,
    write_json,
};

mod json_scripts;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    initialize_logger(cli.debug)?;

    let Command::TagAuthors {
        file,
        out_file,
        platform,
        token,
    } = cli.command;

    let file_path = Path::new(&file);

    if !file_path.exists() {
        return Err(eyre!(format!("file path does not exist: {}", file)));
    }

    let content = fs::read_to_string(file_path).await?;

    let mut packages: Vec<SerializableReleasablePackage> =
        serde_json::from_str(&content)?;

    let client: &dyn PlatformClient = match platform {
        Platform::Slack => &SlackClient::new(token)?,
    };

    let map = client.get_user_name_tag_hash().await?;

    let match_paths: Vec<_> = map.keys().cloned().collect();

    for package in packages.iter_mut() {
        // set release notes to empty since we will be modifying commits
        // which will need to be recompiled into notes by the user via
        // releasaurus show notes --file <file_path>
        package.release.notes = "".into();

        for commit in package.release.commits.iter_mut() {
            // TODO: It probably makes more sense to move this into each client
            // implementation. For example, slack requires additional scope
            // permissions to access a user's email, but this might not be
            // the case for other platforms.

            // fuzzy match on use name using nucleo_matcher
            let mut matcher = Matcher::new(Config::DEFAULT.match_paths());

            let mut matches = Pattern::parse(
                &commit.author_name,
                CaseMatching::Ignore,
                Normalization::Smart,
            )
            .match_list(&match_paths, &mut matcher);

            matches.sort_by_key(|&(_, count)| Reverse(count));

            if let Some((name, score)) = matches.first()
                && let Some(tag) = map.get(*name)
            {
                println!(
                    "found name match! --> {name} : platform_tag={tag}, score={score}"
                );

                commit.author_name = tag.to_string();
            }
        }
    }

    let json = serde_json::json!(packages);
    write_json(&json, &out_file).await?;

    Ok(())
}
