use log::*;
use regex::Regex;

use crate::{
    analyzer::{commit::Commit, group::GroupParser, release::Release},
    forge::request::ForgeCommit,
};

/// Add a parsed commit to the release and update the release SHA and
/// timestamp to reflect the latest commit.
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

/// Normalize changelog formatting by replacing consecutive blank lines (3+)
/// with double newlines and trimming whitespace.
pub fn strip_extra_lines(changelog: &str) -> String {
    let pattern = Regex::new(r"\n{3,}").unwrap();
    pattern.replace_all(changelog, "\n\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::{group::GroupParser, release::Release},
        forge::request::ForgeCommit,
    };

    #[test]
    fn test_update_release_with_commit() {
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let forge_commit1 = ForgeCommit {
            id: "commit1".to_string(),
            link: "https://example.com/commit/commit1".to_string(),
            author_name: "Author 1".to_string(),
            author_email: "author1@example.com".to_string(),
            merge_commit: false,
            message: "fix: first commit".to_string(),
            timestamp: 1640995200,
        };

        let forge_commit2 = ForgeCommit {
            id: "commit2".to_string(),
            link: "https://example.com/commit/commit2".to_string(),
            author_name: "Author 2".to_string(),
            author_email: "author2@example.com".to_string(),
            merge_commit: true,
            message: "feat: second commit".to_string(),
            timestamp: 1640995300,
        };

        update_release_with_commit(&group_parser, &mut release, &forge_commit1);
        update_release_with_commit(&group_parser, &mut release, &forge_commit2);

        // Should have 2 commits
        assert_eq!(release.commits.len(), 2);

        // SHA should be from the last commit
        assert_eq!(release.sha, "commit2");

        // Timestamp should be from the last commit
        assert_eq!(release.timestamp, 1640995300);
    }

    #[test]
    fn test_strip_extra_lines_removes_triple_newlines() {
        let input = "Line 1\n\n\nLine 2";
        let expected = "Line 1\n\nLine 2";
        let result = strip_extra_lines(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_strip_extra_lines_removes_many_newlines() {
        let input = "Line 1\n\n\n\n\n\n\nLine 2";
        let expected = "Line 1\n\nLine 2";
        let result = strip_extra_lines(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_strip_extra_lines_preserves_single_newlines() {
        let input = "Line 1\nLine 2\nLine 3";
        let expected = "Line 1\nLine 2\nLine 3";
        let result = strip_extra_lines(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_strip_extra_lines_preserves_double_newlines() {
        let input = "Line 1\n\nLine 2\n\nLine 3";
        let expected = "Line 1\n\nLine 2\n\nLine 3";
        let result = strip_extra_lines(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_strip_extra_lines_handles_empty_string() {
        let input = "";
        let expected = "";
        let result = strip_extra_lines(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_strip_extra_lines_handles_only_newlines() {
        let input = "\n\n\n\n";
        let expected = "";
        let result = strip_extra_lines(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_strip_extra_lines_trims_leading_and_trailing() {
        let input = "\n\n\nLine 1\n\n\nLine 2\n\n\n";
        let expected = "Line 1\n\nLine 2";
        let result = strip_extra_lines(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_strip_extra_lines_real_changelog_example() {
        let input = r#"# Changelog


## [1.0.0] - 2022-01-01


### Features

- Added new feature



### Bug Fixes

- Fixed bug 1


- Fixed bug 2



## [0.9.0] - 2021-12-01



### Features

- Initial release"#;

        let result = strip_extra_lines(input);

        // Should not contain any triple newlines
        assert!(!result.contains("\n\n\n"));

        // Should still contain double newlines for proper formatting
        assert!(result.contains("\n\n"));

        // Should not be empty
        assert!(!result.is_empty());
    }
}
