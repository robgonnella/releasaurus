use color_eyre::eyre::eyre;
use glob::Pattern;
use log::*;
use regex::Regex;
use std::{path::Path, sync::LazyLock};

pub static HEADER_START_TAG: &str = "<!--releasaurus_header_start-->";
pub static HEADER_END_TAG: &str = "<!--releasaurus_header_end-->";

pub static FOOTER_START_TAG: &str = "<!--releasaurus_footer_start-->";
pub static FOOTER_END_TAG: &str = "<!--releasaurus_footer_end-->";

pub static HEADER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        format!("(?ms)(?<header>{HEADER_START_TAG}.*{HEADER_END_TAG})")
            .as_str(),
    )
    .unwrap()
});

pub static FOOTER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        format!("(?ms)(?<footer>{FOOTER_START_TAG}.*{FOOTER_END_TAG})")
            .as_str(),
    )
    .unwrap()
});

use crate::{
    analyzer::{commit::Commit, groups::GroupParser, release::Release},
    result::Result,
};

/// Process package path into glob patterns for git operations.
pub fn process_package_path(
    repo_path: &str,
    package_relative_path: &str,
) -> Result<Vec<Pattern>> {
    info!("processing package path: {repo_path}/{package_relative_path}");

    let path = Path::new(repo_path).join(package_relative_path);

    // make sure it's a valid directory
    if !path.is_dir() {
        return Err(eyre!(
            "package path is not a valid directory: {}",
            path.to_string_lossy()
        ));
    }

    // now that it's validated we only need to use relative path for git-cliff
    let mut package_path = package_relative_path.to_string();

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

    package_path = package_path
        .strip_prefix("./")
        .unwrap_or(&package_path)
        .to_string();

    // return vec of glob Pattern or None
    let pattern = Pattern::new(&package_path)?;

    Ok(vec![pattern])
}

/// Update release with parsed commit information.
pub fn update_release_with_commit(
    group_parser: &GroupParser,
    link_base: &str,
    release: &mut Release,
    git_commit: &git2::Commit,
) {
    // create git_cliff commit from git2 commit
    let commit = Commit::parse_git2_commit(group_parser, link_base, git_commit);
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
    // set release commit - this will keep getting updated until we
    // get to the last commit in the release, which will be a tag
    release.sha = commit_id;
    release.timestamp = git_commit.time().seconds();
}

/// Replace changelog header section with custom header template.
pub fn replace_header(changelog: &str, header: Option<String>) -> String {
    if header.is_none() {
        return changelog.to_string();
    }

    let new_header = format!(
        "{HEADER_START_TAG}\n{}\n---\n{HEADER_END_TAG}",
        header.unwrap()
    );

    if let Some(captures) = HEADER_REGEX.captures(changelog)
        && let Some(_header_value) = captures.name("header")
    {
        return HEADER_REGEX.replace_all(changelog, new_header).to_string();
    }

    format!("{new_header}\n{changelog}")
}

/// Replace changelog footer section with custom footer template.
pub fn replace_footer(changelog: &str, footer: Option<String>) -> String {
    if footer.is_none() {
        return changelog.to_string();
    }

    let new_footer = format!(
        "{FOOTER_START_TAG}\n---\n{}\n{FOOTER_END_TAG}",
        footer.unwrap()
    );

    if let Some(captures) = FOOTER_REGEX.captures(changelog)
        && let Some(_foooter_value) = captures.name("footer")
    {
        return FOOTER_REGEX.replace_all(changelog, new_footer).to_string();
    }

    format!("{changelog}\n{new_footer}")
}

/// Remove excessive blank lines from changelog content.
pub fn strip_extra_lines(changelog: &str) -> String {
    let pattern = Regex::new(r"\n{3,}").unwrap();
    pattern.replace_all(changelog, "\n\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_for_invalid_package_path() {
        let package_path = "./file.rs";
        let result = process_package_path(".", package_path);
        assert!(result.is_err());
    }

    #[test]
    fn processes_valid_package_path_with_slash() {
        let package_path = "./";

        let expected_pattern = Pattern::new("**/*").unwrap();

        let result = process_package_path(".", package_path);

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

        let expected_pattern = Pattern::new("**/*").unwrap();

        let result = process_package_path(".", package_path);

        assert!(
            result.is_ok(),
            "failed to process package path for valid directory"
        );

        let patterns = result.unwrap();

        assert_eq!(patterns[0], expected_pattern)
    }
}
