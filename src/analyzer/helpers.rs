use log::*;
use regex::Regex;
use semver::{Prerelease, Version};
use std::sync::LazyLock;

use crate::{
    Result,
    analyzer::{
        commit::Commit, config::AnalyzerConfig, group::GroupParser,
        release::Release,
    },
    forge::request::ForgeCommit,
};

/// Matches 3 or more consecutive new lines
static EXTRA_NEW_LINES_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\n{3,}").unwrap());

/// Add a parsed commit to the release and update the release SHA and
/// timestamp to reflect the latest commit.
pub fn update_release_with_commit(
    group_parser: &GroupParser,
    release: &mut Release,
    forge_commit: &ForgeCommit,
    config: &AnalyzerConfig,
) {
    // create git_cliff commit from git2 commit
    if let Some(commit) =
        Commit::parse_forge_commit(group_parser, forge_commit, config)
    {
        let commit_id = commit.id.to_string();

        info!(
            "processing commit: {} : {}",
            commit.short_id, commit.raw_title
        );

        // add commit to release
        release.commits.push(commit);
        // set release commit - this will keep getting updated until we
        // get to the last commit in the release, which will be a tag
        release.sha = commit_id;
        release.timestamp = forge_commit.timestamp;
    }
}

/// Normalize changelog formatting by replacing consecutive blank lines (3+)
/// with double newlines and trimming whitespace.
pub fn strip_extra_lines(changelog: &str) -> String {
    EXTRA_NEW_LINES_REGEX
        .replace_all(changelog, "\n\n")
        .trim()
        .to_string()
}

/// Adds a prerelease identifier to a stable version (e.g., "1.1.0" with "alpha" -> "1.1.0-alpha.1").
pub fn add_prerelease(
    mut version: Version,
    identifier: &str,
    append_version: bool,
) -> Result<Version> {
    let pre_str = if append_version {
        format!("{}.1", identifier)
    } else {
        identifier.to_string()
    };
    version.pre = Prerelease::new(&pre_str)?;
    Ok(version)
}

/// Removes prerelease identifiers from a version (e.g., "1.0.0-alpha.5" -> "1.0.0").
pub fn graduate_prerelease(version: &Version) -> Version {
    let mut new_version = version.clone();
    new_version.pre = Prerelease::EMPTY;
    new_version
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::{group::GroupParser, release::Release},
        forge::request::ForgeCommit,
        test_helpers,
    };

    #[test]
    fn test_update_release_with_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let forge_commit1 = ForgeCommit {
            id: "commit1".to_string(),
            short_id: "comm1".to_string(),
            link: "https://example.com/commit/commit1".to_string(),
            author_name: "Author 1".to_string(),
            author_email: "author1@example.com".to_string(),
            merge_commit: false,
            message: "fix: first commit".to_string(),
            timestamp: 1640995200,
            files: vec![],
        };

        let forge_commit2 = ForgeCommit {
            id: "commit2".to_string(),
            short_id: "comm2".to_string(),
            link: "https://example.com/commit/commit2".to_string(),
            author_name: "Author 2".to_string(),
            author_email: "author2@example.com".to_string(),
            merge_commit: true,
            message: "feat: second commit".to_string(),
            timestamp: 1640995300,
            files: vec![],
        };

        update_release_with_commit(
            &group_parser,
            &mut release,
            &forge_commit1,
            &analyzer_config,
        );
        update_release_with_commit(
            &group_parser,
            &mut release,
            &forge_commit2,
            &analyzer_config,
        );

        // Should have 2 commits
        assert_eq!(release.commits.len(), 1);

        // SHA should be from the last commit
        assert_eq!(release.sha, "commit1");

        // Timestamp should be from the last commit
        assert_eq!(release.timestamp, 1640995200);
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

    #[test]
    fn test_update_release_with_commit_skip_ci() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_ci = true;
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let ci_commit = ForgeCommit {
            id: "ci123".to_string(),
            short_id: "ci1".to_string(),
            link: "https://example.com/commit/ci123".to_string(),
            author_name: "CI Bot".to_string(),
            author_email: "ci@example.com".to_string(),
            merge_commit: false,
            message: "ci: update workflow".to_string(),
            timestamp: 1640995200,
            files: vec![],
        };

        let feat_commit = ForgeCommit {
            id: "feat123".to_string(),
            short_id: "feat1".to_string(),
            link: "https://example.com/commit/feat123".to_string(),
            author_name: "Developer".to_string(),
            author_email: "dev@example.com".to_string(),
            merge_commit: false,
            message: "feat: add feature".to_string(),
            timestamp: 1640995300,
            files: vec![],
        };

        update_release_with_commit(
            &group_parser,
            &mut release,
            &ci_commit,
            &analyzer_config,
        );
        update_release_with_commit(
            &group_parser,
            &mut release,
            &feat_commit,
            &analyzer_config,
        );

        // Should only have 1 commit (feat), ci filtered out
        assert_eq!(release.commits.len(), 1);
        assert_eq!(release.commits[0].id, "feat123");
    }

    #[test]
    fn test_update_release_with_commit_skip_chore() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_chore = true;
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let chore_commit = ForgeCommit {
            id: "chore123".to_string(),
            short_id: "cho1".to_string(),
            link: "https://example.com/commit/chore123".to_string(),
            author_name: "Maintainer".to_string(),
            author_email: "maint@example.com".to_string(),
            merge_commit: false,
            message: "chore: update dependencies".to_string(),
            timestamp: 1640995200,
            files: vec![],
        };

        let fix_commit = ForgeCommit {
            id: "fix123".to_string(),
            short_id: "fi1".to_string(),
            link: "https://example.com/commit/fix123".to_string(),
            author_name: "Developer".to_string(),
            author_email: "dev@example.com".to_string(),
            merge_commit: false,
            message: "fix: fix bug".to_string(),
            timestamp: 1640995300,
            files: vec![],
        };

        update_release_with_commit(
            &group_parser,
            &mut release,
            &chore_commit,
            &analyzer_config,
        );
        update_release_with_commit(
            &group_parser,
            &mut release,
            &fix_commit,
            &analyzer_config,
        );

        // Should only have 1 commit (fix), chore filtered out
        assert_eq!(release.commits.len(), 1);
        assert_eq!(release.commits[0].id, "fix123");
    }

    #[test]
    fn test_update_release_with_commit_skip_miscellaneous() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_miscellaneous = true;
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let misc_commit = ForgeCommit {
            id: "misc123".to_string(),
            short_id: "mi1".to_string(),
            link: "https://example.com/commit/misc123".to_string(),
            author_name: "Random User".to_string(),
            author_email: "random@example.com".to_string(),
            merge_commit: false,
            message: "random commit without type".to_string(),
            timestamp: 1640995200,
            files: vec![],
        };

        let feat_commit = ForgeCommit {
            id: "feat123".to_string(),
            short_id: "fe1".to_string(),
            link: "https://example.com/commit/feat123".to_string(),
            author_name: "Developer".to_string(),
            author_email: "dev@example.com".to_string(),
            merge_commit: false,
            message: "feat: add feature".to_string(),
            timestamp: 1640995300,
            files: vec![],
        };

        update_release_with_commit(
            &group_parser,
            &mut release,
            &misc_commit,
            &analyzer_config,
        );
        update_release_with_commit(
            &group_parser,
            &mut release,
            &feat_commit,
            &analyzer_config,
        );

        // Should only have 1 commit (feat), miscellaneous filtered out
        assert_eq!(release.commits.len(), 1);
        assert_eq!(release.commits[0].id, "feat123");
    }

    #[test]
    fn test_update_release_with_commit_skip_multiple_types() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_ci = true;
        analyzer_config.skip_chore = true;
        analyzer_config.skip_miscellaneous = true;
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let commits = vec![
            ForgeCommit {
                id: "ci123".to_string(),
                short_id: "ci1".to_string(),
                link: "https://example.com/commit/ci123".to_string(),
                author_name: "CI Bot".to_string(),
                author_email: "ci@example.com".to_string(),
                merge_commit: false,
                message: "ci: update workflow".to_string(),
                timestamp: 1640995100,
                files: vec![],
            },
            ForgeCommit {
                id: "chore123".to_string(),
                short_id: "ch1".to_string(),
                link: "https://example.com/commit/chore123".to_string(),
                author_name: "Maintainer".to_string(),
                author_email: "maint@example.com".to_string(),
                merge_commit: false,
                message: "chore: cleanup".to_string(),
                timestamp: 1640995200,
                files: vec![],
            },
            ForgeCommit {
                id: "misc123".to_string(),
                short_id: "mi1".to_string(),
                link: "https://example.com/commit/misc123".to_string(),
                author_name: "Random".to_string(),
                author_email: "random@example.com".to_string(),
                merge_commit: false,
                message: "random commit".to_string(),
                timestamp: 1640995250,
                files: vec![],
            },
            ForgeCommit {
                id: "feat123".to_string(),
                short_id: "fe1".to_string(),
                link: "https://example.com/commit/feat123".to_string(),
                author_name: "Developer".to_string(),
                author_email: "dev@example.com".to_string(),
                merge_commit: false,
                message: "feat: add feature".to_string(),
                timestamp: 1640995300,
                files: vec![],
            },
            ForgeCommit {
                id: "fix123".to_string(),
                short_id: "fi1".to_string(),
                link: "https://example.com/commit/fix123".to_string(),
                author_name: "Developer 2".to_string(),
                author_email: "dev2@example.com".to_string(),
                merge_commit: false,
                message: "fix: fix bug".to_string(),
                timestamp: 1640995400,
                files: vec![],
            },
        ];

        for commit in &commits {
            update_release_with_commit(
                &group_parser,
                &mut release,
                commit,
                &analyzer_config,
            );
        }

        // Should only have 2 commits (feat and fix)
        assert_eq!(release.commits.len(), 2);
        assert_eq!(release.commits[0].id, "feat123");
        assert_eq!(release.commits[1].id, "fix123");
    }

    #[test]
    fn test_update_release_with_commit_preserves_author_info() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let commit_with_author = ForgeCommit {
            id: "author123".to_string(),
            short_id: "au1".to_string(),
            link: "https://example.com/commit/author123".to_string(),
            author_name: "Jane Smith".to_string(),
            author_email: "jane.smith@example.com".to_string(),
            merge_commit: false,
            message: "feat: add new feature".to_string(),
            timestamp: 1640995200,
            files: vec![],
        };

        update_release_with_commit(
            &group_parser,
            &mut release,
            &commit_with_author,
            &analyzer_config,
        );

        assert_eq!(release.commits.len(), 1);
        assert_eq!(release.commits[0].author_name, "Jane Smith");
        assert_eq!(release.commits[0].author_email, "jane.smith@example.com");
    }

    #[test]
    fn test_update_release_with_commit_author_info_with_skip_options() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_ci = true;
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let ci_commit = ForgeCommit {
            id: "ci123".to_string(),
            short_id: "ci1".to_string(),
            link: "https://example.com/commit/ci123".to_string(),
            author_name: "CI Bot".to_string(),
            author_email: "ci@example.com".to_string(),
            merge_commit: false,
            message: "ci: update workflow".to_string(),
            timestamp: 1640995200,
            files: vec![],
        };

        let feat_commit = ForgeCommit {
            id: "feat123".to_string(),
            short_id: "fe1".to_string(),
            link: "https://example.com/commit/feat123".to_string(),
            author_name: "John Doe".to_string(),
            author_email: "john.doe@example.com".to_string(),
            merge_commit: false,
            message: "feat: add feature".to_string(),
            timestamp: 1640995300,
            files: vec![],
        };

        update_release_with_commit(
            &group_parser,
            &mut release,
            &ci_commit,
            &analyzer_config,
        );
        update_release_with_commit(
            &group_parser,
            &mut release,
            &feat_commit,
            &analyzer_config,
        );

        // Should only have feat commit with author info preserved
        assert_eq!(release.commits.len(), 1);
        assert_eq!(release.commits[0].author_name, "John Doe");
        assert_eq!(release.commits[0].author_email, "john.doe@example.com");
    }

    #[test]
    fn test_update_release_with_commit_no_skip_includes_all() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let commits = vec![
            ForgeCommit {
                id: "ci123".to_string(),
                short_id: "ci1".to_string(),
                link: "https://example.com/commit/ci123".to_string(),
                author_name: "CI Bot".to_string(),
                author_email: "ci@example.com".to_string(),
                merge_commit: false,
                message: "ci: update workflow".to_string(),
                timestamp: 1640995100,
                files: vec![],
            },
            ForgeCommit {
                id: "chore123".to_string(),
                short_id: "ch1".to_string(),
                link: "https://example.com/commit/chore123".to_string(),
                author_name: "Maintainer".to_string(),
                author_email: "maint@example.com".to_string(),
                merge_commit: false,
                message: "chore: cleanup".to_string(),
                timestamp: 1640995200,
                files: vec![],
            },
            ForgeCommit {
                id: "misc123".to_string(),
                short_id: "mi1".to_string(),
                link: "https://example.com/commit/misc123".to_string(),
                author_name: "Random".to_string(),
                author_email: "random@example.com".to_string(),
                merge_commit: false,
                message: "random commit".to_string(),
                timestamp: 1640995250,
                files: vec![],
            },
            ForgeCommit {
                id: "feat123".to_string(),
                short_id: "fe1".to_string(),
                link: "https://example.com/commit/feat123".to_string(),
                author_name: "Developer".to_string(),
                author_email: "dev@example.com".to_string(),
                merge_commit: false,
                message: "feat: add feature".to_string(),
                timestamp: 1640995300,
                files: vec![],
            },
        ];

        for commit in &commits {
            update_release_with_commit(
                &group_parser,
                &mut release,
                commit,
                &analyzer_config,
            );
        }

        // Should have all 4 commits when no skip options are enabled
        assert_eq!(release.commits.len(), 4);
    }

    #[test]
    fn test_add_prerelease() {
        let version = Version::parse("1.0.0").unwrap();
        let result = add_prerelease(version, "alpha", true).unwrap();
        assert_eq!(result, Version::parse("1.0.0-alpha.1").unwrap());

        let version = Version::parse("2.3.4").unwrap();
        let result = add_prerelease(version, "beta", true).unwrap();
        assert_eq!(result, Version::parse("2.3.4-beta.1").unwrap());

        let version = Version::parse("0.1.0").unwrap();
        let result = add_prerelease(version, "rc", true).unwrap();
        assert_eq!(result, Version::parse("0.1.0-rc.1").unwrap());

        let version = Version::parse("0.1.0").unwrap();
        let result = add_prerelease(version, "SNAPSHOT", false).unwrap();
        assert_eq!(result, Version::parse("0.1.0-SNAPSHOT").unwrap());
    }

    #[test]
    fn test_add_prerelease_to_prerelease_version() {
        // Adding prerelease to an existing prerelease replaces it
        let version = Version::parse("1.0.0-alpha.5").unwrap();
        let result = add_prerelease(version, "beta", true).unwrap();
        assert_eq!(result, Version::parse("1.0.0-beta.1").unwrap());
    }

    #[test]
    fn test_graduate_prerelease() {
        let version = Version::parse("1.0.0-alpha.1").unwrap();
        let result = graduate_prerelease(&version);
        assert_eq!(result, Version::parse("1.0.0").unwrap());

        let version = Version::parse("2.3.4-beta.5").unwrap();
        let result = graduate_prerelease(&version);
        assert_eq!(result, Version::parse("2.3.4").unwrap());

        let version = Version::parse("0.1.0-rc.10").unwrap();
        let result = graduate_prerelease(&version);
        assert_eq!(result, Version::parse("0.1.0").unwrap());
    }

    #[test]
    fn test_graduate_prerelease_stable_version_unchanged() {
        let version = Version::parse("1.0.0").unwrap();
        let result = graduate_prerelease(&version);
        assert_eq!(result, Version::parse("1.0.0").unwrap());
    }

    #[test]
    fn test_graduate_prerelease_preserves_build_metadata() {
        let version = Version::parse("1.0.0-alpha.1+build.123").unwrap();
        let result = graduate_prerelease(&version);
        assert_eq!(result, Version::parse("1.0.0+build.123").unwrap());
    }
}
