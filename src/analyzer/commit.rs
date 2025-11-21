use git_conventional::Commit as ConventionalCommit;
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
    pub group: Group,
    pub scope: Option<String>,
    pub message: String,
    pub body: Option<String>,
    pub link: String,
    pub breaking: bool,
    pub breaking_description: Option<String>,
    pub merge_commit: bool,
    pub timestamp: i64,
    pub author_name: String,
    pub author_email: String,
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
        let merge_commit = forge_commit.merge_commit;
        let raw_message = forge_commit.message.clone();
        let timestamp = forge_commit.timestamp;
        let parsed = ConventionalCommit::parse(raw_message.trim_end());
        let link = forge_commit.link.clone();

        match parsed {
            Ok(cc) => {
                let mut commit = Self {
                    id: commit_id,
                    scope: cc.scope().map(|s| s.to_string()),
                    message: cc.description().to_string(),
                    body: cc.body().map(|b| b.to_string()),
                    merge_commit,
                    breaking: cc.breaking(),
                    breaking_description: cc
                        .breaking_description()
                        .map(|d| d.to_string()),
                    raw_message: raw_message.to_string(),
                    group: Group::default(),
                    link,
                    timestamp,
                    author_name,
                    author_email,
                };
                commit.group = group_parser.parse(&commit);
                if commit.group == Group::Ci && config.skip_ci {
                    return None;
                }
                if commit.group == Group::Chore && config.skip_chore {
                    return None;
                }
                if commit.group == Group::Miscellaneous
                    && config.skip_miscellaneous
                {
                    return None;
                }
                Some(commit)
            }
            Err(_) => {
                if config.skip_miscellaneous {
                    return None;
                }
                let split = raw_message
                    .split_once("\n")
                    .map(|(m, b)| (m.to_string(), b.to_string()));

                let (message, body) = match split {
                    Some((m, b)) => {
                        if b.is_empty() {
                            (m, None)
                        } else {
                            (m, Some(b))
                        }
                    }
                    None => (raw_message.to_string(), None),
                };

                Some(Self {
                    id: commit_id,
                    scope: None,
                    message,
                    body,
                    merge_commit,
                    breaking: false,
                    breaking_description: None,
                    raw_message: raw_message.to_string(),
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
        ForgeCommit {
            id: id.to_string(),
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
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "add new user authentication");
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
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "add OAuth2 support");
        assert_eq!(commit.body, None);
        assert!(!commit.breaking);
        assert_eq!(commit.author_name, "Jane Smith");
        assert_eq!(commit.author_email, "jane@example.com");
    }

    #[test]
    fn test_parse_conventional_fix_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "resolve null pointer exception");
        assert!(!commit.breaking);
    }

    #[test]
    fn test_parse_breaking_change_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "redesign user API");
        assert_eq!(commit.body, None);
        assert!(commit.breaking);
        assert_eq!(
            commit.breaking_description,
            Some("The user API has been completely redesigned".to_string())
        );
    }

    #[test]
    fn test_parse_commit_with_body() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "add user registration");
        assert_eq!(commit.body, Some("This feature allows new users to register\nwith email verification.".to_string()));
        assert!(!commit.breaking);
    }

    #[test]
    fn test_parse_non_conventional_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "Update user authentication logic");
        assert_eq!(commit.body, None);
        assert!(!commit.breaking);
        assert_eq!(commit.breaking_description, None);
    }

    #[test]
    fn test_parse_non_conventional_commit_with_body() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "Update database schema");
        assert_eq!(commit.body, Some("\nAdded new indexes for better performance\nand updated user table structure.".to_string()));
    }

    #[test]
    fn test_parse_merge_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "Merge pull request #123 from feature/auth");
        assert_eq!(commit.author_name, "GitHub");
        assert_eq!(commit.author_email, "noreply@github.com");
    }

    #[test]
    fn test_parse_empty_message() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "");
        assert_eq!(commit.body, None);
        assert!(!commit.breaking);
    }

    #[test]
    fn test_parse_chore_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "update dependencies");
    }

    #[test]
    fn test_parse_ci_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "update GitHub Actions workflow");
    }

    #[test]
    fn test_parse_docs_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
            commit.message,
            "update README with installation instructions"
        );
    }

    #[test]
    fn test_parse_test_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "add unit tests for user service");
    }

    #[test]
    fn test_parse_refactor_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "simplify authentication logic");
    }

    #[test]
    fn test_parse_perf_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "optimize database queries");
    }

    #[test]
    fn test_parse_style_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "format code with prettier");
    }

    #[test]
    fn test_parse_revert_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "undo breaking changes");
    }

    #[test]
    fn test_parse_commit_with_trailing_whitespace() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "add new feature");
        // Body should be None since it's just whitespace
        assert_eq!(commit.body, None);
    }

    #[test]
    fn test_parse_commit_with_empty_body() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "resolve issue");
        assert_eq!(commit.body, None);
    }

    #[test]
    fn test_parse_breaking_change_with_conventional_format() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "remove deprecated endpoints");
        assert!(commit.breaking);
        assert_eq!(
            commit.breaking_description,
            Some("The old v1 API endpoints have been removed".to_string())
        );
    }

    #[test]
    fn test_metadata_preservation() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "unique123".to_string(),
            link: "https://custom-forge.com/commit/unique123".to_string(),
            author_name: "Custom Author".to_string(),
            author_email: "custom@forge.com".to_string(),
            merge_commit: true,
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
        assert!(commit.merge_commit);
        assert_eq!(commit.timestamp, 9999999999);
        assert_eq!(commit.raw_message, "feat: custom forge commit");
    }

    #[test]
    fn test_non_conventional_single_line() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "Just a simple commit message");
        assert_eq!(commit.body, None);
        assert_eq!(commit.scope, None);
        assert!(!commit.breaking);
        assert_eq!(commit.breaking_description, None);
    }

    #[test]
    fn test_malformed_conventional_commit() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "feat add feature without colon");
        assert_eq!(commit.scope, None);
        assert!(!commit.breaking);
    }

    #[test]
    fn test_all_conventional_commit_types() {
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "breaking feature change");
    }

    #[test]
    fn test_skip_ci_filters_ci_commits() {
        let mut analyzer_config = test_helpers::create_test_analyzer_config();
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
        let mut analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "update github actions workflow");
    }

    #[test]
    fn test_skip_chore_filters_chore_commits() {
        let mut analyzer_config = test_helpers::create_test_analyzer_config();
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
        let mut analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "update dependencies");
    }

    #[test]
    fn test_skip_miscellaneous_filters_non_conventional_commits() {
        let mut analyzer_config = test_helpers::create_test_analyzer_config();
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
        let mut analyzer_config = test_helpers::create_test_analyzer_config();
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
        assert_eq!(commit.message, "random commit message without type");
    }

    #[test]
    fn test_skip_ci_with_different_ci_formats() {
        let mut analyzer_config = test_helpers::create_test_analyzer_config();
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
        let mut analyzer_config = test_helpers::create_test_analyzer_config();
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
        let mut analyzer_config = test_helpers::create_test_analyzer_config();
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
        let analyzer_config = test_helpers::create_test_analyzer_config();
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
