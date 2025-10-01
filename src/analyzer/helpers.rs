use log::*;
use regex::Regex;

use crate::{
    analyzer::{commit::Commit, group::GroupParser, release::Release},
    forge::request::ForgeCommit,
};

/// Update release with parsed commit information.
pub fn update_release_with_commit(
    group_parser: &GroupParser,
    release: &mut Release,
    forge_commit: &ForgeCommit,
) {
    // create git_cliff commit from git2 commit
    let commit = Commit::parse_forge_commit(group_parser, forge_commit);
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
    release.timestamp = forge_commit.timestamp;
}

/// Remove excessive blank lines from changelog content.
pub fn strip_extra_lines(changelog: &str) -> String {
    let pattern = Regex::new(r"\n{3,}").unwrap();
    pattern.replace_all(changelog, "\n\n").trim().to_string()
}
