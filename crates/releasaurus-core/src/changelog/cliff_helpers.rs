use color_eyre::eyre::{Result, eyre};
use glob::Pattern;
use log::*;
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value};
use std::path::Path;

use crate::{changelog::config::ChangelogConfig, config::Remote};

pub fn process_package_path(package_path: String) -> Result<Vec<Pattern>> {
    let path = Path::new(package_path.as_str());

    if !path.is_dir() {
        return Err(eyre!(
            "package path is not a valid directory: {}",
            path.to_string_lossy()
        ));
    }

    let mut package_path = package_path;

    // include paths only work on with globs
    if package_path.ends_with("*") {
        // if the path provided ends in * return as is
        debug!("using package path {package_path}")
    } else if package_path.ends_with("/") {
        // otherwise if ends in "/" return modified with glob
        package_path = format!("{package_path}**/*").to_string();
        debug!("modified package_path to include glob: {package_path}")
    } else {
        // otherwise return a modified version that adds /**/*
        package_path = format!("{package_path}/**/*").to_string();
        debug!("modified package_path to include directory glob {package_path}")
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
        process_package_path(changelog_config.package.path.clone())?;
    Ok(())
}

pub fn set_config_remote(
    cliff_config: &mut git_cliff_core::config::Config,
    changelog_config: &ChangelogConfig,
) {
    match changelog_config.remote.clone() {
        Remote::Github(remote_config) => {
            cliff_config.remote.github.owner = remote_config.owner;
            cliff_config.remote.github.repo = remote_config.repo;
            cliff_config.remote.github.token =
                Some(remote_config.token.clone());
            cliff_config.remote.github.api_url = remote_config.api_url;
            cliff_config.remote.github.is_custom = true;
        }
        Remote::Gitlab(remote_config) => {
            cliff_config.remote.gitlab.owner = remote_config.owner;
            cliff_config.remote.gitlab.repo = remote_config.repo;
            cliff_config.remote.gitlab.token =
                Some(remote_config.token.clone());
            cliff_config.remote.gitlab.api_url = remote_config.api_url;
            cliff_config.remote.gitlab.is_custom = true;
        }
        Remote::Gitea(remote_config) => {
            cliff_config.remote.gitea.owner = remote_config.owner;
            cliff_config.remote.gitea.repo = remote_config.repo;
            cliff_config.remote.gitea.token = Some(remote_config.token.clone());
            cliff_config.remote.gitea.api_url = remote_config.api_url;
            cliff_config.remote.gitea.is_custom = true;
        }
    }
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

    info!("using tag_prefix: {tag_prefix}");
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
    set_config_remote(&mut cliff_config, &changelog_config);
    set_config_tag_settings(&mut cliff_config, &changelog_config)?;
    set_config_commit_parsers(&mut cliff_config)?;

    Ok(cliff_config)
}

pub fn get_commit_link_for_remote(remote: Remote, commit_id: String) -> String {
    match remote {
        Remote::Github(config) => format!(
            "{}/{}/{}/commit/{}",
            config.base_url, config.owner, config.repo, commit_id
        ),
        Remote::Gitlab(config) => format!(
            "{}/{}/{}/commit/{}",
            config.base_url, config.owner, config.repo, commit_id
        ),
        Remote::Gitea(config) => format!(
            "{}/{}/{}/commit/{}",
            config.base_url, config.owner, config.repo, commit_id
        ),
    }
}

pub fn get_version_link_for_remote(remote: Remote, tag: String) -> String {
    match remote {
        Remote::Github(config) => format!(
            "{}/{}/{}/releases/tag/{}",
            config.base_url, config.owner, config.repo, tag
        ),
        Remote::Gitlab(config) => format!(
            "{}/{}/{}/releases/{}",
            config.base_url, config.owner, config.repo, tag
        ),
        Remote::Gitea(config) => format!(
            "{}/{}/{}/releases/{}",
            config.base_url, config.owner, config.repo, tag
        ),
    }
}

pub fn update_release_with_commit(
    repo_path: String,
    release: &mut git_cliff_core::release::Release,
    git_commit: &git2::Commit,
    remote: Remote,
) {
    // create git_cliff commit from git2 commit
    let mut commit = git_cliff_core::commit::Commit::from(git_commit);

    // add extra link properties for remote
    let mut commit_extra = Map::new();

    commit_extra.insert(
        "link".to_string(),
        Value::String(get_commit_link_for_remote(
            remote.clone(),
            commit.id.clone(),
        )),
    );

    commit.extra = Some(Value::Object(commit_extra));

    let commit_id = commit.id.to_string();
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
        return None;
    }
    // we've found the top of the release!
    // current_version = Some(tag.name.clone());
    release.version = Some(tag.name.to_string());
    release.message.clone_from(&tag.message);
    release.timestamp = Some(git_commit.time().seconds());
    // return the version found
    Some(tag.name.to_string())
}

pub fn add_version_link_and_commit_range_to_release(
    release: &mut git_cliff_core::release::Release,
    remote: Remote,
) {
    // add extra version_link property
    if let Some(version) = release.version.clone() {
        let mut release_extra = Map::new();
        release_extra.insert(
            "version_link".to_string(),
            Value::String(get_version_link_for_remote(remote, version)),
        );
        release.extra = Some(Value::Object(release_extra));
    }

    // Set the commit ranges for all releases
    if !release.commits.is_empty() {
        release.commit_range = Some(git_cliff_core::commit::Range::new(
            release.commits.first().unwrap(),
            release.commits.last().unwrap(),
        ))
    }
}
