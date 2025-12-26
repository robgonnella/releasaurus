use log::*;
use regex::Regex;
use semver::{Prerelease, Version};
use std::{borrow::Cow, sync::LazyLock};

use crate::{
    Result,
    analyzer::{
        commit::Commit, config::AnalyzerConfig, group::GroupParser,
        release::Release,
    },
    config::prerelease::PrereleaseStrategy,
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
    strategy: PrereleaseStrategy,
) -> Result<Version> {
    // Use Cow to avoid allocation for Static strategy
    let pre_str: Cow<str> = if matches!(strategy, PrereleaseStrategy::Versioned)
    {
        Cow::Owned(format!("{}.1", identifier))
    } else {
        Cow::Borrowed(identifier)
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
        config::prerelease::PrereleaseStrategy,
        forge::request::ForgeCommitBuilder,
    };

    #[test]
    fn test_update_release_with_commit() {
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let forge_commit1 = ForgeCommitBuilder::default()
            .id("commit1")
            .short_id("comm1")
            .link("https://example.com/commit/commit1")
            .author_name("Author 1")
            .author_email("author1@example.com")
            .merge_commit(false)
            .message("fix: first commit")
            .timestamp(1640995200)
            .files(vec![])
            .build()
            .unwrap();

        let forge_commit2 = ForgeCommitBuilder::default()
            .id("commit2")
            .short_id("comm2")
            .link("https://example.com/commit/commit2")
            .author_name("Author 2")
            .author_email("author2@example.com")
            .merge_commit(true)
            .message("feat: second commit")
            .timestamp(1640995300)
            .files(vec![])
            .build()
            .unwrap();

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
        let analyzer_config = AnalyzerConfig {
            skip_ci: true,
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let ci_commit = ForgeCommitBuilder::default()
            .id("ci123")
            .short_id("ci1")
            .link("https://example.com/commit/ci123")
            .author_name("CI Bot")
            .author_email("ci@example.com")
            .merge_commit(false)
            .message("ci: update workflow")
            .timestamp(1640995100)
            .files(vec![])
            .build()
            .unwrap();

        let feat_commit = ForgeCommitBuilder::default()
            .id("feat123")
            .short_id("feat1")
            .link("https://example.com/commit/feat123")
            .author_name("Developer")
            .author_email("dev@example.com")
            .merge_commit(false)
            .message("feat: add feature")
            .timestamp(1640995200)
            .files(vec![])
            .build()
            .unwrap();

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
        let analyzer_config = AnalyzerConfig {
            skip_chore: true,
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let chore_commit = ForgeCommitBuilder::default()
            .id("chore123")
            .short_id("cho1")
            .link("https://example.com/commit/chore123")
            .author_name("Maintainer")
            .author_email("maint@example.com")
            .merge_commit(false)
            .message("chore: update deps")
            .timestamp(1640995100)
            .files(vec![])
            .build()
            .unwrap();

        let fix_commit = ForgeCommitBuilder::default()
            .id("fix123")
            .short_id("fi1")
            .link("https://example.com/commit/fix123")
            .author_name("Developer")
            .author_email("dev@example.com")
            .merge_commit(false)
            .message("fix: bug fix")
            .timestamp(1640995200)
            .files(vec![])
            .build()
            .unwrap();

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
        let analyzer_config = AnalyzerConfig {
            skip_miscellaneous: true,
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let misc_commit = ForgeCommitBuilder::default()
            .id("misc123")
            .short_id("mi1")
            .link("https://example.com/commit/misc123")
            .author_name("Random")
            .author_email("random@example.com")
            .merge_commit(false)
            .message("random message")
            .timestamp(1640995100)
            .files(vec![])
            .build()
            .unwrap();

        let feat_commit = ForgeCommitBuilder::default()
            .id("feat123")
            .short_id("fe1")
            .link("https://example.com/commit/feat123")
            .author_name("Developer")
            .author_email("dev@example.com")
            .merge_commit(false)
            .message("feat: add feature")
            .timestamp(1640995200)
            .files(vec![])
            .build()
            .unwrap();

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
        let analyzer_config = AnalyzerConfig {
            skip_ci: true,
            skip_chore: true,
            skip_miscellaneous: true,
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let commits = vec![
            ForgeCommitBuilder::default()
                .id("ci123")
                .short_id("ci1")
                .link("https://example.com/commit/ci123")
                .author_name("CI Bot")
                .author_email("ci@example.com")
                .merge_commit(false)
                .message("ci: update workflow")
                .timestamp(1640995100)
                .files(vec![])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("chore123")
                .short_id("ch1")
                .link("https://example.com/commit/chore123")
                .author_name("Maintainer")
                .author_email("maint@example.com")
                .merge_commit(false)
                .message("chore: update deps")
                .timestamp(1640995200)
                .files(vec![])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("misc123")
                .short_id("mi1")
                .link("https://example.com/commit/misc123")
                .author_name("Random")
                .author_email("random@example.com")
                .merge_commit(false)
                .message("random message")
                .timestamp(1640995300)
                .files(vec![])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("feat123")
                .short_id("fe1")
                .link("https://example.com/commit/feat123")
                .author_name("Developer")
                .author_email("dev@example.com")
                .merge_commit(false)
                .message("feat: add feature")
                .timestamp(1640995400)
                .files(vec![])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("fix123")
                .short_id("fi1")
                .link("https://example.com/commit/fix123")
                .author_name("Developer")
                .author_email("dev@example.com")
                .merge_commit(false)
                .message("fix: bug fix")
                .timestamp(1640995500)
                .files(vec![])
                .build()
                .unwrap(),
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let commit_with_author = ForgeCommitBuilder::default()
            .id("author123")
            .short_id("au1")
            .link("https://example.com/commit/author123")
            .author_name("Jane Smith")
            .author_email("jane.smith@example.com")
            .merge_commit(false)
            .message("feat: new feature")
            .timestamp(1640995100)
            .files(vec![])
            .build()
            .unwrap();

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
        let analyzer_config = AnalyzerConfig {
            skip_ci: true,
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let ci_commit = ForgeCommitBuilder::default()
            .id("ci123")
            .short_id("ci1")
            .link("https://example.com/commit/ci123")
            .author_name("CI Bot")
            .author_email("ci@example.com")
            .merge_commit(false)
            .message("ci: update workflow")
            .timestamp(1640995100)
            .files(vec![])
            .build()
            .unwrap();

        let feat_commit = ForgeCommitBuilder::default()
            .id("feat123")
            .short_id("fe1")
            .link("https://example.com/commit/feat123")
            .author_name("John Doe")
            .author_email("john.doe@example.com")
            .merge_commit(false)
            .message("feat: add feature")
            .timestamp(1640995200)
            .files(vec![])
            .build()
            .unwrap();

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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let mut release = Release::default();

        let commits = vec![
            ForgeCommitBuilder::default()
                .id("ci123")
                .short_id("ci1")
                .link("https://example.com/commit/ci123")
                .author_name("CI Bot")
                .author_email("ci@example.com")
                .merge_commit(false)
                .message("ci: update workflow")
                .timestamp(1640995100)
                .files(vec![])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("chore123")
                .short_id("ch1")
                .link("https://example.com/commit/chore123")
                .author_name("Maintainer")
                .author_email("maint@example.com")
                .merge_commit(false)
                .message("chore: update deps")
                .timestamp(1640995200)
                .files(vec![])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("misc123")
                .short_id("mi1")
                .link("https://example.com/commit/misc123")
                .author_name("Random")
                .author_email("random@example.com")
                .merge_commit(false)
                .message("random message")
                .timestamp(1640995300)
                .files(vec![])
                .build()
                .unwrap(),
            ForgeCommitBuilder::default()
                .id("feat123")
                .short_id("fe1")
                .link("https://example.com/commit/feat123")
                .author_name("Developer")
                .author_email("dev@example.com")
                .merge_commit(false)
                .message("feat: add feature")
                .timestamp(1640995400)
                .files(vec![])
                .build()
                .unwrap(),
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
    fn test_add_versioned_prerelease() {
        let version = Version::parse("1.0.0").unwrap();
        let result =
            add_prerelease(version, "alpha", PrereleaseStrategy::Versioned)
                .unwrap();
        assert_eq!(result, Version::parse("1.0.0-alpha.1").unwrap());
    }

    #[test]
    fn test_add_static_prerelease() {
        let version = Version::parse("0.1.0").unwrap();
        let result =
            add_prerelease(version, "SNAPSHOT", PrereleaseStrategy::Static)
                .unwrap();
        assert_eq!(result, Version::parse("0.1.0-SNAPSHOT").unwrap());
    }

    #[test]
    fn test_change_prerelease_suffix() {
        // Changing prerelease suffix to an existing prerelease replaces it
        let version = Version::parse("1.0.0-alpha.5").unwrap();
        let result =
            add_prerelease(version, "beta", PrereleaseStrategy::Versioned)
                .unwrap();
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
