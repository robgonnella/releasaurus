use color_eyre::eyre::{Result, eyre};
use glob::Pattern;
use log::*;
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value};
use std::{path::Path, sync::LazyLock};

static RELEASE_NOTES_START_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#\s\[.*\]\(.*\)\s-\s\d{4}-\d{2}-\d{2}").unwrap()
});

use crate::analyzer::config::AnalyzerConfig;

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
    analyzer_config: &AnalyzerConfig,
) -> Result<()> {
    cliff_config.changelog.body = analyzer_config.body.clone();
    cliff_config.changelog.header = analyzer_config.header.clone();
    cliff_config.changelog.footer = analyzer_config.footer.clone();
    cliff_config.changelog.trim = true;
    cliff_config.git.conventional_commits = true;
    cliff_config.git.filter_unconventional = false;
    cliff_config.git.protect_breaking_commits = true;
    cliff_config.git.require_conventional = false;
    cliff_config.git.include_paths =
        process_package_path(&analyzer_config.package_path)?;
    Ok(())
}

pub fn set_config_tag_settings(
    cliff_config: &mut git_cliff_core::config::Config,
    analyzer_config: &AnalyzerConfig,
) -> Result<()> {
    let mut tag_prefix = "v".to_string();

    if let Some(prefix) = analyzer_config.tag_prefix.clone() {
        tag_prefix = prefix;
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
    analyzer_config: AnalyzerConfig,
) -> Result<git_cliff_core::config::Config> {
    let mut cliff_config = git_cliff_core::embed::EmbeddedConfig::parse()?;

    set_config_basic_settings(&mut cliff_config, &analyzer_config)?;
    set_config_tag_settings(&mut cliff_config, &analyzer_config)?;
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
    info!("processing tag: {}", tag.name);
    // we only care about releases for this specific package
    if let Some(re) = tag_pattern.clone()
        && !re.is_match(&tag.name)
    {
        info!(
            "tag does not match pattern: skipping: tag: {:#?}, pattern: {:#?}",
            tag.name, tag_pattern,
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

pub fn parse_projected_release_notes(changelog: &str) -> String {
    let notes: Vec<&str> = RELEASE_NOTES_START_LINE
        .split(changelog.trim())
        .map(|c| c.trim())
        .collect();
    notes[1].to_string()
}

pub fn strip_trailing_previous_release(changelog: &str) -> String {
    let starting_flag = Regex::new(r"(?m)^#\s").unwrap();
    let stripped: Vec<&str> = starting_flag
        .splitn(changelog.trim(), 3)
        .map(|c| c.trim())
        .collect();
    format!("# {}\n\n", stripped[1])
}

#[cfg(test)]
mod tests {
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

        let analyzer_config = AnalyzerConfig::default();

        let result =
            set_config_tag_settings(&mut cliff_config, &analyzer_config);

        assert!(result.is_ok(), "failed to set config tag settings");

        assert!(cliff_config.git.tag_pattern.is_some());

        let tag_pattern = cliff_config.git.tag_pattern.unwrap();

        assert!(tag_pattern.is_match("v1.0.0"));
    }

    #[test]
    fn set_config_tag_settings_uses_prefix_option() {
        let prefix = "prefix".to_string();

        let mut cliff_config =
            git_cliff_core::embed::EmbeddedConfig::parse().unwrap();

        let analyzer_config = AnalyzerConfig {
            tag_prefix: Some(prefix),
            ..AnalyzerConfig::default()
        };

        let result =
            set_config_tag_settings(&mut cliff_config, &analyzer_config);

        assert!(result.is_ok(), "failed to set config tag settings");

        assert!(cliff_config.git.tag_pattern.is_some());

        let tag_pattern = cliff_config.git.tag_pattern.unwrap();

        assert!(!tag_pattern.is_match("v1.0.0"));
        assert!(!tag_pattern.is_match("test-v1.0.0"));
        assert!(tag_pattern.is_match("prefix-v1.0.0"));
    }
}
