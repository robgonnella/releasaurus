use git_conventional::Commit as ConventionalCommit;
use log::*;
use serde::Serialize;

use crate::{
    analyzer::{
        config::AnalyzerConfig,
        group::{Group, GroupParser},
    },
    forge::request::ForgeCommit,
};

/// Structured commit with parsed conventional commit fields, author
/// metadata, and changelog categorization.
#[derive(Debug, Clone, Serialize)]
pub struct Commit {
    pub id: String,
    pub short_id: String,
    pub group: Group,
    pub scope: Option<String>,
    pub title: String,
    pub body: Option<String>,
    pub link: String,
    pub breaking: bool,
    pub breaking_description: Option<String>,
    pub merge_commit: bool,
    pub timestamp: i64,
    pub author_name: String,
    pub author_email: String,
    pub raw_title: String,
    pub raw_message: String,
}

impl Commit {
    /// Parse forge commit into structured format, extracting conventional
    /// commit fields or falling back to plain message parsing.
    pub fn parse_forge_commit(
        group_parser: &GroupParser,
        forge_commit: &ForgeCommit,
        config: &AnalyzerConfig,
    ) -> Option<Self> {
        let author_name = forge_commit.author_name.clone();
        let author_email = forge_commit.author_email.clone();
        let commit_id = forge_commit.id.clone();
        let short_id = forge_commit.short_id.clone();
        let merge_commit = forge_commit.merge_commit;
        let raw_message = forge_commit.message.clone();
        let timestamp = forge_commit.timestamp;
        let link = forge_commit.link.clone();

        let split_msg = forge_commit
            .message
            .split_once("\n")
            .map(|(m, b)| (m.to_string(), b.to_string()));

        let (raw_title, raw_body) = match split_msg {
            Some((t, b)) => {
                if b.is_empty() {
                    (t.trim().to_string(), None)
                } else {
                    (t.trim().to_string(), Some(b.trim().to_string()))
                }
            }
            None => (raw_message.to_string(), None),
        };

        let parsed = ConventionalCommit::parse(raw_message.trim_end());

        match parsed {
            Ok(cc) => {
                let mut commit = Self {
                    id: commit_id,
                    short_id,
                    scope: cc.scope().map(|s| s.to_string()),
                    title: cc.description().trim().to_string(),
                    body: cc.body().map(|b| b.trim().to_string()),
                    merge_commit,
                    breaking: cc.breaking(),
                    breaking_description: cc
                        .breaking_description()
                        .map(|d| d.to_string()),
                    raw_title,
                    raw_message,
                    group: Group::default(),
                    link,
                    timestamp,
                    author_name,
                    author_email,
                };
                commit.group = group_parser.parse(&commit);
                if commit.group == Group::Ci && config.skip_ci {
                    debug!(
                        "omitting ci commit: {} : {}",
                        commit.short_id, commit.raw_title
                    );
                    return None;
                }
                if commit.group == Group::Chore && config.skip_chore {
                    debug!(
                        "omitting chore commit: {} : {}",
                        commit.short_id, commit.raw_title
                    );
                    return None;
                }
                if commit.group == Group::Miscellaneous
                    && config.skip_miscellaneous
                {
                    debug!(
                        "omitting miscellaneous commit: {} : {}",
                        commit.short_id, commit.raw_title
                    );
                    return None;
                }
                if commit.merge_commit && config.skip_merge_commits {
                    debug!(
                        "omitting merge commit: {} : {}",
                        commit.short_id, commit.raw_title
                    );
                    return None;
                }
                if config.skip_release_commits
                    && let Some(matcher) = config.release_commit_matcher.clone()
                    && matcher.is_match(&commit.raw_title)
                {
                    debug!(
                        "omitting release commit: {} : {}",
                        commit.short_id, commit.raw_title
                    );

                    return None;
                }

                Some(commit)
            }
            Err(_) => {
                if config.skip_miscellaneous {
                    return None;
                }

                if merge_commit && config.skip_merge_commits {
                    debug!("omitting merge commit: {short_id} : {raw_title}");
                    return None;
                }

                Some(Self {
                    id: commit_id,
                    short_id,
                    scope: None,
                    title: raw_title.clone(),
                    body: raw_body,
                    merge_commit,
                    breaking: false,
                    breaking_description: None,
                    raw_title,
                    raw_message,
                    group: Group::default(),
                    link,
                    timestamp,
                    author_name,
                    author_email,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyzer::group::GroupParser, test_helpers};

    fn create_test_forge_commit(
        id: &str,
        message: &str,
        author_name: &str,
        author_email: &str,
        timestamp: i64,
        merge_commit: bool,
    ) -> ForgeCommit {
        let short_id = id.split("").take(8).collect::<Vec<&str>>().join("");
        ForgeCommit {
            id: id.to_string(),
            short_id,
            link: format!("https://github.com/example/repo/commit/{}", id),
            author_name: author_name.to_string(),
            author_email: author_email.to_string(),
            merge_commit,
            message: message.to_string(),
            timestamp,
            files: vec![],
        }
    }

    #[test]
    fn test_parse_conventional_feat_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "abc123",
            "feat: add new user authentication",
            "John Doe",
            "john@example.com",
            1640995200,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.id, "abc123");
        assert_eq!(commit.group, Group::Feat);
        assert_eq!(commit.scope, None);
        assert_eq!(commit.title, "add new user authentication");
        assert_eq!(commit.body, None);
        assert_eq!(
            commit.link,
            "https://github.com/example/repo/commit/abc123"
        );
        assert!(!commit.breaking);
        assert_eq!(commit.breaking_description, None);
        assert!(!commit.merge_commit);
        assert_eq!(commit.timestamp, 1640995200);
        assert_eq!(commit.author_name, "John Doe");
        assert_eq!(commit.author_email, "john@example.com");
        assert_eq!(commit.raw_message, "feat: add new user authentication");
    }

    #[test]
    fn test_parse_conventional_feat_commit_with_scope() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "def456",
            "feat(auth): add OAuth2 support",
            "Jane Smith",
            "jane@example.com",
            1640995300,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.id, "def456");
        assert_eq!(commit.group, Group::Feat);
        assert_eq!(commit.scope, Some("auth".to_string()));
        assert_eq!(commit.title, "add OAuth2 support");
        assert_eq!(commit.body, None);
        assert!(!commit.breaking);
        assert_eq!(commit.author_name, "Jane Smith");
        assert_eq!(commit.author_email, "jane@example.com");
    }

    #[test]
    fn test_parse_conventional_fix_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "ghi789",
            "fix: resolve null pointer exception",
            "Bob Johnson",
            "bob@example.com",
            1640995400,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Fix);
        assert_eq!(commit.title, "resolve null pointer exception");
        assert!(!commit.breaking);
    }

    #[test]
    fn test_parse_breaking_change_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "jkl012",
            "feat!: redesign user API\n\nBREAKING CHANGE: The user API has been completely redesigned",
            "Alice Brown",
            "alice@example.com",
            1640995500,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Breaking);
        assert_eq!(commit.title, "redesign user API");
        assert_eq!(commit.body, None);
        assert!(commit.breaking);
        assert_eq!(
            commit.breaking_description,
            Some("The user API has been completely redesigned".to_string())
        );
    }

    #[test]
    fn test_parse_commit_with_body() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let message = "feat: add user registration\n\nThis feature allows new users to register\nwith email verification.";
        let forge_commit = create_test_forge_commit(
            "mno345",
            message,
            "Charlie Wilson",
            "charlie@example.com",
            1640995600,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Feat);
        assert_eq!(commit.title, "add user registration");
        assert_eq!(commit.body, Some("This feature allows new users to register\nwith email verification.".to_string()));
        assert!(!commit.breaking);
    }

    #[test]
    fn test_parse_non_conventional_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "pqr678",
            "Update user authentication logic",
            "David Lee",
            "david@example.com",
            1640995700,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Miscellaneous);
        assert_eq!(commit.scope, None);
        assert_eq!(commit.title, "Update user authentication logic");
        assert_eq!(commit.body, None);
        assert!(!commit.breaking);
        assert_eq!(commit.breaking_description, None);
    }

    #[test]
    fn test_parse_non_conventional_commit_with_body() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let message = "Update database schema\n\nAdded new indexes for better performance\nand updated user table structure.";
        let forge_commit = create_test_forge_commit(
            "stu901",
            message,
            "Eva Martinez",
            "eva@example.com",
            1640995800,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Miscellaneous);
        assert_eq!(commit.title, "Update database schema");
        assert_eq!(commit.body, Some("Added new indexes for better performance\nand updated user table structure.".to_string()));
    }

    #[test]
    fn test_parses_and_omits_merge_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "vwx234",
            "Merge pull request #123 from feature/auth",
            "GitHub",
            "noreply@github.com",
            1640995900,
            true,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        );

        assert!(commit.is_none());
    }

    #[test]
    fn test_parses_and_includes_merge_commit() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_merge_commits = false;

        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "vwx234",
            "Merge pull request #123 from feature/auth",
            "GitHub",
            "noreply@github.com",
            1640995900,
            true,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Miscellaneous);
        assert!(commit.merge_commit);
        assert_eq!(commit.title, "Merge pull request #123 from feature/auth");
        assert_eq!(commit.author_name, "GitHub");
        assert_eq!(commit.author_email, "noreply@github.com");
    }

    #[test]
    fn test_parses_and_omits_release_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "vwx234",
            "chore(main): release test-package test-package-v1.0.0",
            "GitHub",
            "noreply@github.com",
            1640995900,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        );

        assert!(commit.is_none());
    }

    #[test]
    fn test_parses_and_includes_release_commit() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_release_commits = false;

        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "vwx234",
            "chore(main): release test-package test-package-v1.0.0",
            "GitHub",
            "noreply@github.com",
            1640995900,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Chore);
        assert!(!commit.merge_commit);
        assert_eq!(commit.title, "release test-package test-package-v1.0.0");
        assert_eq!(commit.author_name, "GitHub");
        assert_eq!(commit.author_email, "noreply@github.com");
    }

    #[test]
    fn test_parse_empty_message() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "yz567",
            "",
            "Test User",
            "test@example.com",
            1641000000,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Miscellaneous);
        assert_eq!(commit.title, "");
        assert_eq!(commit.body, None);
        assert!(!commit.breaking);
    }

    #[test]
    fn test_parse_chore_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "abc890",
            "chore: update dependencies",
            "Maintainer",
            "maintainer@example.com",
            1641000100,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Chore);
        assert_eq!(commit.title, "update dependencies");
    }

    #[test]
    fn test_parse_ci_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "def123",
            "ci: update GitHub Actions workflow",
            "DevOps",
            "devops@example.com",
            1641000200,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Ci);
        assert_eq!(commit.title, "update GitHub Actions workflow");
    }

    #[test]
    fn test_parse_docs_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "ghi456",
            "doc: update README with installation instructions",
            "Technical Writer",
            "writer@example.com",
            1641000300,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Doc);
        assert_eq!(
            commit.title,
            "update README with installation instructions"
        );
    }

    #[test]
    fn test_parse_test_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "jkl789",
            "test: add unit tests for user service",
            "QA Engineer",
            "qa@example.com",
            1641000400,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Test);
        assert_eq!(commit.title, "add unit tests for user service");
    }

    #[test]
    fn test_parse_refactor_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "mno012",
            "refactor: simplify authentication logic",
            "Senior Dev",
            "senior@example.com",
            1641000500,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Refactor);
        assert_eq!(commit.title, "simplify authentication logic");
    }

    #[test]
    fn test_parse_perf_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "pqr345",
            "perf: optimize database queries",
            "Performance Engineer",
            "perf@example.com",
            1641000600,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Perf);
        assert_eq!(commit.title, "optimize database queries");
    }

    #[test]
    fn test_parse_style_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "stu678",
            "style: format code with prettier",
            "Style Bot",
            "bot@example.com",
            1641000700,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Style);
        assert_eq!(commit.title, "format code with prettier");
    }

    #[test]
    fn test_parse_revert_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "vwx901",
            "revert: undo breaking changes",
            "Emergency Fix",
            "emergency@example.com",
            1641000800,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Revert);
        assert_eq!(commit.title, "undo breaking changes");
    }

    #[test]
    fn test_parse_commit_with_trailing_whitespace() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "xyz234",
            "feat: add new feature   \n\n  ",
            "Whitespace User",
            "whitespace@example.com",
            1641000900,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Feat);
        assert_eq!(commit.title, "add new feature");
        // Body should be None since it's just whitespace
        assert_eq!(commit.body, None);
    }

    #[test]
    fn test_parse_commit_with_empty_body() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "abc567",
            "fix: resolve issue\n\n",
            "Fix User",
            "fix@example.com",
            1641001000,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Fix);
        assert_eq!(commit.title, "resolve issue");
        assert_eq!(commit.body, None);
    }

    #[test]
    fn test_parse_breaking_change_with_conventional_format() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let message = "feat(api)!: remove deprecated endpoints\n\nBREAKING CHANGE: The old v1 API endpoints have been removed";
        let forge_commit = create_test_forge_commit(
            "def890",
            message,
            "API Team",
            "api@example.com",
            1641001100,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Breaking);
        assert_eq!(commit.scope, Some("api".to_string()));
        assert_eq!(commit.title, "remove deprecated endpoints");
        assert!(commit.breaking);
        assert_eq!(
            commit.breaking_description,
            Some("The old v1 API endpoints have been removed".to_string())
        );
    }

    #[test]
    fn test_metadata_preservation() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "unique123".to_string(),
            short_id: "uni".to_string(),
            link: "https://custom-forge.com/commit/unique123".to_string(),
            author_name: "Custom Author".to_string(),
            author_email: "custom@forge.com".to_string(),
            merge_commit: false,
            message: "feat: custom forge commit".to_string(),
            timestamp: 9999999999,
            files: vec![],
        };

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        // Verify all metadata is preserved correctly
        assert_eq!(commit.id, "unique123");
        assert_eq!(commit.link, "https://custom-forge.com/commit/unique123");
        assert_eq!(commit.author_name, "Custom Author");
        assert_eq!(commit.author_email, "custom@forge.com");
        assert!(!commit.merge_commit);
        assert_eq!(commit.timestamp, 9999999999);
        assert_eq!(commit.raw_message, "feat: custom forge commit");
    }

    #[test]
    fn test_non_conventional_single_line() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "single123",
            "Just a simple commit message",
            "Simple User",
            "simple@example.com",
            1641001200,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        assert_eq!(commit.group, Group::Miscellaneous);
        assert_eq!(commit.title, "Just a simple commit message");
        assert_eq!(commit.body, None);
        assert_eq!(commit.scope, None);
        assert!(!commit.breaking);
        assert_eq!(commit.breaking_description, None);
    }

    #[test]
    fn test_malformed_conventional_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "malformed123",
            "feat add feature without colon",
            "Malformed User",
            "malformed@example.com",
            1641001300,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        // Should be treated as non-conventional since it lacks the colon
        assert_eq!(commit.group, Group::Miscellaneous);
        assert_eq!(commit.title, "feat add feature without colon");
        assert_eq!(commit.scope, None);
        assert!(!commit.breaking);
    }

    #[test]
    fn test_all_conventional_commit_types() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();

        let test_cases = vec![
            ("feat: new feature", Group::Feat),
            ("fix: bug fix", Group::Fix),
            ("chore: maintenance", Group::Chore),
            ("doc: documentation", Group::Doc),
            ("style: formatting", Group::Style),
            ("refactor: code cleanup", Group::Refactor),
            ("perf: performance improvement", Group::Perf),
            ("test: add tests", Group::Test),
            ("revert: revert changes", Group::Revert),
            ("ci: CI/CD updates", Group::Ci),
        ];

        for (message, expected_group) in test_cases {
            let forge_commit = create_test_forge_commit(
                "test123",
                message,
                "Test User",
                "test@example.com",
                1641000000,
                false,
            );

            let commit = Commit::parse_forge_commit(
                &group_parser,
                &forge_commit,
                &analyzer_config,
            )
            .unwrap();
            assert_eq!(
                commit.group, expected_group,
                "Failed for message: {}",
                message
            );
        }
    }

    #[test]
    fn test_breaking_takes_precedence_over_type() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "breaking123",
            "feat!: breaking feature change",
            "Breaking User",
            "breaking@example.com",
            1641000000,
            false,
        );

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        // Breaking should take precedence over feat
        assert_eq!(commit.group, Group::Breaking);
        assert!(commit.breaking);
        assert_eq!(commit.title, "breaking feature change");
    }

    #[test]
    fn test_skip_ci_filters_ci_commits() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_ci = true;
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "ci123",
            "ci: update github actions workflow",
            "CI User",
            "ci@example.com",
            1641000000,
            false,
        );

        let result = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        );

        // Should return None when skip_ci is true
        assert!(result.is_none());
    }

    #[test]
    fn test_skip_ci_false_includes_ci_commits() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_ci = false;
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "ci123",
            "ci: update github actions workflow",
            "CI User",
            "ci@example.com",
            1641000000,
            false,
        );

        let result = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        );

        // Should return Some when skip_ci is false
        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(commit.group, Group::Ci);
        assert_eq!(commit.title, "update github actions workflow");
    }

    #[test]
    fn test_skip_chore_filters_chore_commits() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_chore = true;
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "chore123",
            "chore: update dependencies",
            "Chore User",
            "chore@example.com",
            1641000000,
            false,
        );

        let result = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        );

        // Should return None when skip_chore is true
        assert!(result.is_none());
    }

    #[test]
    fn test_skip_chore_false_includes_chore_commits() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_chore = false;
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "chore123",
            "chore: update dependencies",
            "Chore User",
            "chore@example.com",
            1641000000,
            false,
        );

        let result = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        );

        // Should return Some when skip_chore is false
        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(commit.group, Group::Chore);
        assert_eq!(commit.title, "update dependencies");
    }

    #[test]
    fn test_skip_miscellaneous_filters_non_conventional_commits() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_miscellaneous = true;
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "misc123",
            "random commit message without type",
            "Random User",
            "random@example.com",
            1641000000,
            false,
        );

        let result = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        );

        // Should return None when skip_miscellaneous is true
        assert!(result.is_none());
    }

    #[test]
    fn test_skip_miscellaneous_false_includes_non_conventional_commits() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_miscellaneous = false;
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "misc123",
            "random commit message without type",
            "Random User",
            "random@example.com",
            1641000000,
            false,
        );

        let result = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        );

        // Should return Some when skip_miscellaneous is false
        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(commit.group, Group::Miscellaneous);
        assert_eq!(commit.title, "random commit message without type");
    }

    #[test]
    fn test_skip_ci_with_different_ci_formats() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_ci = true;
        let group_parser = GroupParser::new();

        // Test various CI commit formats
        let test_cases = vec![
            "ci: update workflow",
            "ci(github): add new action",
            "ci(gitlab): fix pipeline",
        ];

        for message in test_cases {
            let forge_commit = create_test_forge_commit(
                "ci123",
                message,
                "CI User",
                "ci@example.com",
                1641000000,
                false,
            );

            let result = Commit::parse_forge_commit(
                &group_parser,
                &forge_commit,
                &analyzer_config,
            );

            assert!(result.is_none(), "Expected None for message: {}", message);
        }
    }

    #[test]
    fn test_skip_chore_with_different_chore_formats() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_chore = true;
        let group_parser = GroupParser::new();

        // Test various chore commit formats
        let test_cases = vec![
            "chore: update deps",
            "chore(deps): bump version",
            "chore(lint): fix warnings",
        ];

        for message in test_cases {
            let forge_commit = create_test_forge_commit(
                "chore123",
                message,
                "Chore User",
                "chore@example.com",
                1641000000,
                false,
            );

            let result = Commit::parse_forge_commit(
                &group_parser,
                &forge_commit,
                &analyzer_config,
            );

            assert!(result.is_none(), "Expected None for message: {}", message);
        }
    }

    #[test]
    fn test_skip_options_do_not_affect_other_types() {
        let mut analyzer_config =
            test_helpers::create_test_analyzer_config(None);
        analyzer_config.skip_ci = true;
        analyzer_config.skip_chore = true;
        analyzer_config.skip_miscellaneous = true;
        let group_parser = GroupParser::new();

        // Test that other commit types are not affected
        let test_cases = vec![
            ("feat: add feature", Group::Feat),
            ("fix: fix bug", Group::Fix),
            ("docs: update docs", Group::Doc),
            ("test: add tests", Group::Test),
            ("refactor: refactor code", Group::Refactor),
            ("perf: improve performance", Group::Perf),
            ("style: format code", Group::Style),
            ("revert: revert change", Group::Revert),
        ];

        for (message, expected_group) in test_cases {
            let forge_commit = create_test_forge_commit(
                "commit123",
                message,
                "Test User",
                "test@example.com",
                1641000000,
                false,
            );

            let result = Commit::parse_forge_commit(
                &group_parser,
                &forge_commit,
                &analyzer_config,
            );

            assert!(result.is_some(), "Expected Some for message: {}", message);
            let commit = result.unwrap();
            assert_eq!(
                commit.group, expected_group,
                "Wrong group for message: {}",
                message
            );
        }
    }

    #[test]
    fn test_author_information_preserved() {
        let analyzer_config = test_helpers::create_test_analyzer_config(None);
        let group_parser = GroupParser::new();
        let forge_commit = create_test_forge_commit(
            "author123",
            "feat: add feature",
            "John Doe",
            "john.doe@example.com",
            1641000000,
            false,
        );

        let result = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        );

        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(commit.author_name, "John Doe");
        assert_eq!(commit.author_email, "john.doe@example.com");
    }
}
