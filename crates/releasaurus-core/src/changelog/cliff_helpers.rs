use color_eyre::eyre::{Result, eyre};
use glob::Pattern;
use log::*;
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value};
use std::path::Path;

use crate::changelog::config::ChangelogConfig;

pub fn process_package_path(package_path: &str) -> Result<Vec<Pattern>> {
    info!("processing package path: {package_path}");

    let path = Path::new(package_path);

    if !path.is_dir() {
        return Err(eyre!(
            "package path is not a valid directory: {}",
            path.to_string_lossy()
        ));
    }

    let mut package_path = package_path.to_string();

    // include paths only work on with globs
    if package_path.ends_with("/") {
        // otherwise if ends in "/" return modified with glob
        package_path = format!("{package_path}**/*").to_string();
        info!("modified package_path to include glob: {package_path}")
    } else {
        // otherwise return a modified version that adds /**/*
        package_path = format!("{package_path}/**/*").to_string();
        info!("modified package_path to include directory glob {package_path}")
    };

    // return vec of glob Pattern or None
    let pattern = Pattern::new(&package_path)?;

    Ok(vec![pattern])
}

pub fn set_config_basic_settings(
    cliff_config: &mut git_cliff_core::config::Config,
    changelog_config: &ChangelogConfig,
) -> Result<()> {
    cliff_config.changelog.body = changelog_config.body.clone();
    cliff_config.changelog.header = changelog_config.header.clone();
    cliff_config.changelog.footer = changelog_config.footer.clone();
    cliff_config.changelog.trim = true;
    cliff_config.git.conventional_commits = true;
    cliff_config.git.filter_unconventional = false;
    cliff_config.git.protect_breaking_commits = true;
    cliff_config.git.require_conventional = false;
    cliff_config.git.include_paths =
        process_package_path(&changelog_config.package.path)?;
    Ok(())
}

pub fn set_config_tag_settings(
    cliff_config: &mut git_cliff_core::config::Config,
    changelog_config: &ChangelogConfig,
) -> Result<()> {
    let mut tag_prefix = "v".to_string();

    if !changelog_config.package.name.is_empty() {
        tag_prefix = format!("{}-v", changelog_config.package.name);
        info!("set tag prefix for package name: {tag_prefix}");
    }

    if let Some(prefix) = changelog_config.package.tag_prefix.clone() {
        tag_prefix = prefix;
        info!("set tag prefix to provided option: {tag_prefix}");
    }

    info!("configuring tag prefix: {tag_prefix}");
    let regex_prefix = format!(r"^{}", tag_prefix);
    let re = Regex::new(&regex_prefix)?;
    cliff_config.git.tag_pattern = Some(re);
    cliff_config.bump.initial_tag = Some(format!("{}0.1.0", tag_prefix));
    Ok(())
}

// adds "Breaking Change" group to beginning of default commit-parsers list
pub fn set_config_commit_parsers(
    cliff_config: &mut git_cliff_core::config::Config,
) -> Result<()> {
    info!("updating commit parsers");
    let group_number_re = Regex::new(r"\d{1,2}\s-")?;
    let mut group_id = 0;

    // bump up all group numbers by 1 to allow breaking changes
    // group to be inserted at position 0
    for parser in cliff_config.git.commit_parsers.iter_mut() {
        if let Some(ref group) = parser.group
            && group_number_re.is_match(group)
        {
            group_id += 1;
            let new_group =
                group_number_re.replace(group, format!("{group_id} -"));
            parser.group = Some(new_group.to_string());
        }
    }

    // create breaking changes commit parser
    let mut breaking_change_parser =
        git_cliff_core::config::CommitParser::default();

    let br_message_re = Regex::new(r"^.+!:")?;
    let br_footer_re = RegexBuilder::new(r"breaking-?change:")
        .case_insensitive(true)
        .build()?;

    breaking_change_parser.message = Some(br_message_re);
    breaking_change_parser.footer = Some(br_footer_re);
    breaking_change_parser.group =
        Some("<!-- 0 -->❌ Breaking Changes".to_string());

    // adds breaking changes group as first parser
    cliff_config
        .git
        .commit_parsers
        .insert(0, breaking_change_parser);

    Ok(())
}

pub fn get_cliff_config(
    changelog_config: ChangelogConfig,
) -> Result<git_cliff_core::config::Config> {
    let mut cliff_config = git_cliff_core::embed::EmbeddedConfig::parse()?;

    set_config_basic_settings(&mut cliff_config, &changelog_config)?;
    set_config_tag_settings(&mut cliff_config, &changelog_config)?;
    set_config_commit_parsers(&mut cliff_config)?;

    Ok(cliff_config)
}

pub fn update_release_with_commit(
    repo_path: String,
    release: &mut git_cliff_core::release::Release,
    git_commit: &git2::Commit,
) {
    // create git_cliff commit from git2 commit
    let commit = git_cliff_core::commit::Commit::from(git_commit);
    let commit_id = commit.id.to_string();
    let lines = commit
        .message
        .split("\n")
        .map(|l| l.to_string())
        .collect::<Vec<String>>();
    let title = lines.first();

    if let Some(t) = title {
        let short_sha =
            commit_id.split("").take(8).collect::<Vec<&str>>().join("");
        info!("processing commit: {} : {}", short_sha, t);
    }
    // add commit to release
    release.commits.push(commit);
    release.repository = Some(repo_path);
    // set release commit - this will keep getting updated until we
    // get to the last commit in the release, which will be a tag
    release.commit_id = Some(commit_id);
}

pub fn process_tag_for_release(
    release: &mut git_cliff_core::release::Release,
    git_commit: &git2::Commit,
    tag: &git_cliff_core::tag::Tag,
    tag_pattern: Option<Regex>,
) -> Option<String> {
    // we only care about releases for this specific package
    if let Some(re) = tag_pattern.clone()
        && !re.is_match(&tag.name)
    {
        info!(
            "tag does not match pattern: skipping: tag: {:#?}, pattern: {:#?}",
            re, tag.name
        );
        return None;
    }
    // we've found the top of the release!
    // current_version = Some(tag.name.clone());
    info!("identified previous release version: {}", tag.name);
    release.version = Some(tag.name.to_string());
    release.message.clone_from(&tag.message);
    release.timestamp = Some(git_commit.time().seconds());
    // return the version found
    Some(tag.name.to_string())
}

pub fn add_link_base_and_commit_range_to_release(
    release: &mut git_cliff_core::release::Release,
    commit_link_base_url: &str,
    release_link_base_url: &str,
) {
    // add extra link properties
    let mut release_extra = Map::new();

    release_extra.insert(
        "release_link_base".to_string(),
        Value::String(release_link_base_url.to_string()),
    );

    release_extra.insert(
        "commit_link_base".to_string(),
        Value::String(commit_link_base_url.to_string()),
    );

    release.extra = Some(Value::Object(release_extra));

    // Set the commit ranges for all releases
    if !release.commits.is_empty() {
        release.commit_range = Some(git_cliff_core::commit::Range::new(
            release.commits.first().unwrap(),
            release.commits.last().unwrap(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::changelog::config::PackageConfig;

    use super::*;

    #[test]
    fn errors_for_invalid_package_path() {
        let package_path = "./file.rs";
        let result = process_package_path(package_path);
        assert!(result.is_err());
    }

    #[test]
    fn processes_valid_package_path_with_slash() {
        let package_path = "./";

        let expected_pattern = Pattern::new("./**/*").unwrap();

        let result = process_package_path(package_path);

        assert!(
            result.is_ok(),
            "failed to process package path for valid directory"
        );

        let patterns = result.unwrap();

        assert_eq!(patterns[0], expected_pattern)
    }

    #[test]
    fn processes_valid_package_path_without_slash() {
        let package_path = ".";

        let expected_pattern = Pattern::new("./**/*").unwrap();

        let result = process_package_path(package_path);

        assert!(
            result.is_ok(),
            "failed to process package path for valid directory"
        );

        let patterns = result.unwrap();

        assert_eq!(patterns[0], expected_pattern)
    }

    #[test]
    fn set_config_tag_settings_uses_default() {
        let mut cliff_config =
            git_cliff_core::embed::EmbeddedConfig::parse().unwrap();

        let changelog_config = ChangelogConfig::default();

        let result =
            set_config_tag_settings(&mut cliff_config, &changelog_config);

        assert!(result.is_ok(), "failed to set config tag settings");

        assert!(cliff_config.git.tag_pattern.is_some());

        let tag_pattern = cliff_config.git.tag_pattern.unwrap();

        assert!(tag_pattern.is_match("v1.0.0"));
    }

    #[test]
    fn set_config_tag_settings_uses_package_name() {
        let package_name = "test".to_string();

        let mut cliff_config =
            git_cliff_core::embed::EmbeddedConfig::parse().unwrap();

        let changelog_config = ChangelogConfig {
            package: PackageConfig {
                name: package_name,
                ..PackageConfig::default()
            },
            ..ChangelogConfig::default()
        };

        let result =
            set_config_tag_settings(&mut cliff_config, &changelog_config);

        assert!(result.is_ok(), "failed to set config tag settings");

        assert!(cliff_config.git.tag_pattern.is_some());

        let tag_pattern = cliff_config.git.tag_pattern.unwrap();

        assert!(!tag_pattern.is_match("v1.0.0"));
        assert!(tag_pattern.is_match("test-v1.0.0"));
    }

    #[test]
    fn set_config_tag_settings_uses_prefix_option() {
        let prefix = "prefix".to_string();

        let mut cliff_config =
            git_cliff_core::embed::EmbeddedConfig::parse().unwrap();

        let changelog_config = ChangelogConfig {
            package: PackageConfig {
                name: "test".into(),
                tag_prefix: Some(prefix),
                ..PackageConfig::default()
            },
            ..ChangelogConfig::default()
        };

        let result =
            set_config_tag_settings(&mut cliff_config, &changelog_config);

        assert!(result.is_ok(), "failed to set config tag settings");

        assert!(cliff_config.git.tag_pattern.is_some());

        let tag_pattern = cliff_config.git.tag_pattern.unwrap();

        assert!(!tag_pattern.is_match("v1.0.0"));
        assert!(!tag_pattern.is_match("test-v1.0.0"));
        assert!(tag_pattern.is_match("prefix-v1.0.0"));
    }
}
