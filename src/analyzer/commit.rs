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
#[derive(Debug, Clone, Default, Serialize)]
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
    use regex::Regex;

    use super::*;
    use crate::analyzer::group::GroupParser;

    #[test]
    fn test_parse_conventional_feat_commit() {
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "abc123".into(),
            message: "feat: add new user authentication".into(),
            author_name: "John Doe".into(),
            author_email: "john@example.com".into(),
            timestamp: 1640995200,
            merge_commit: false,
            ..ForgeCommit::default()
        };

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
        assert_eq!(commit.link, "");
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "def456".into(),
            message: "feat(auth): add OAuth2 support".into(),
            author_name: "Jane Smith".into(),
            author_email: "jane@example.com".into(),
            timestamp: 1640995300,
            merge_commit: false,
            ..ForgeCommit::default()
        };

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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "ghi789".into(),
            message: "fix: resolve null pointer exception".into(),
            author_name: "Bob Johnson".into(),
            author_email: "bob@example.com".into(),
            timestamp: 1640995400,
            merge_commit: false,
            ..ForgeCommit::default()
        };

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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "jkl012".into(),
            message: "feat!: redesign user API\n\nBREAKING CHANGE: The user API has been completely redesigned".into(),
            author_name: "Alice Brown".into(),
            author_email: "alice@example.com".into(),
            timestamp: 1640995500,
            merge_commit: false,
            ..ForgeCommit::default()
        };

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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let message = "feat: add user registration\n\nThis feature allows new users to register\nwith email verification.".to_string();
        let forge_commit = ForgeCommit {
            id: "mno345".into(),
            message,
            author_name: "Charlie Wilson".into(),
            author_email: "charlie@example.com".into(),
            timestamp: 1640995600,
            merge_commit: false,
            ..ForgeCommit::default()
        };

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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "pqr678".into(),
            message: "Update user authentication logic".into(),
            author_name: "David Lee".into(),
            author_email: "david@example.com".into(),
            timestamp: 1640995700,
            merge_commit: false,
            ..ForgeCommit::default()
        };

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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let message = "Update database schema\n\nAdded new indexes for better performance\nand updated user table structure.".to_string();
        let forge_commit = ForgeCommit {
            id: "stu901".into(),
            message,
            author_name: "Eva Martinez".into(),
            author_email: "eva@example.com".into(),
            timestamp: 1640995800,
            merge_commit: false,
            ..ForgeCommit::default()
        };

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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "vwx234".into(),
            message: "Merge pull request #123 from feature/auth".into(),
            author_name: "GitHub".into(),
            author_email: "noreply@github.com".into(),
            timestamp: 1640995900,
            merge_commit: true,
            ..ForgeCommit::default()
        };

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        );

        assert!(commit.is_none());
    }

    #[test]
    fn test_parses_and_includes_merge_commit() {
        let analyzer_config = AnalyzerConfig {
            skip_merge_commits: false,
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "vwx234".into(),
            message: "Merge pull request #123 from feature/auth".into(),
            author_name: "GitHub".into(),
            author_email: "noreply@github.com".into(),
            timestamp: 1640995900,
            merge_commit: true,
            ..ForgeCommit::default()
        };

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
        let analyzer_config = AnalyzerConfig {
            skip_chore: false,
            skip_release_commits: true,
            release_commit_matcher: Some(
                Regex::new(r#"^chore\(main\):\srelease.+"#).unwrap(),
            ),
            tag_prefix: Some("test-package-v".into()),
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "vwx234".into(),
            message: "chore(main): release test-package test-package-v1.0.0"
                .into(),
            author_name: "GitHub".into(),
            author_email: "noreply@github.com".into(),
            timestamp: 1640995900,
            merge_commit: false,
            ..ForgeCommit::default()
        };

        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        );

        assert!(commit.is_none());
    }

    #[test]
    fn test_parses_and_includes_release_commit() {
        let analyzer_config = AnalyzerConfig {
            skip_release_commits: false,
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "vwx234".into(),
            message: "chore(main): release test-package test-package-v1.0.0"
                .into(),
            author_name: "GitHub".into(),
            author_email: "noreply@github.com".into(),
            timestamp: 1640995900,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "yz567".into(),
            message: "".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000000,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "abc890".into(),
            message: "chore: update dependencies".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000100,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "def123".into(),
            message: "ci: update GitHub Actions workflow".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000200,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "ghi456".into(),
            message: "doc: update README with installation instructions".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "jkl789".into(),
            message: "test: add unit tests for user service".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "mno012".into(),
            message: "refactor: simplify authentication logic".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "pqr345".into(),
            message: "perf: optimize database queries".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "stu678".into(),
            message: "style: format code with prettier".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "vwx901".into(),
            message: "revert: undo breaking changes".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "xyz234".into(),
            message: "feat: add new feature   \n\n  ".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "abc567".into(),
            message: "fix: resolve issue\n\n".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let message = "feat(api)!: remove deprecated endpoints\n\nBREAKING CHANGE: The old v1 API endpoints have been removed".to_string();
        let forge_commit = ForgeCommit {
            id: "def890".into(),
            message,
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig::default();
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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "single123".into(),
            message: "Just a simple commit message".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
    fn test_breaking_takes_precedence_over_type() {
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "breaking123".into(),
            message: "feat!: feature change".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
        let commit = Commit::parse_forge_commit(
            &group_parser,
            &forge_commit,
            &analyzer_config,
        )
        .unwrap();

        // Breaking should take precedence over feat
        assert_eq!(commit.group, Group::Breaking);
        assert!(commit.breaking);
        assert_eq!(commit.title, "feature change");
    }

    #[test]
    fn test_skip_ci_filters_ci_commits() {
        let analyzer_config = AnalyzerConfig {
            skip_ci: true,
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "ci123".into(),
            message: "ci: update github actions workflow".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig {
            skip_ci: false,
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "ci123".into(),
            message: "ci: update github actions workflow".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig {
            skip_chore: true,
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "chore123".into(),
            message: "chore: update dependencies".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig {
            skip_chore: false,
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "chore123".into(),
            message: "chore: update dependencies".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig {
            skip_miscellaneous: true,
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "misc123".into(),
            message: "random commit message without type".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
        let analyzer_config = AnalyzerConfig {
            skip_miscellaneous: false,
            ..AnalyzerConfig::default()
        };
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "misc123".into(),
            message: "random commit message without type".into(),
            author_name: "Test User".into(),
            author_email: "test@example.com".into(),
            timestamp: 1641000300,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
    fn test_skip_options_do_not_affect_other_types() {
        let analyzer_config = AnalyzerConfig {
            skip_chore: true,
            skip_ci: true,
            skip_merge_commits: true,
            skip_miscellaneous: true,
            skip_release_commits: true,
            ..AnalyzerConfig::default()
        };
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
            let forge_commit = ForgeCommit {
                id: "commit123".into(),
                message: message.into(),
                author_name: "Test User".into(),
                author_email: "test@example.com".into(),
                timestamp: 1641000300,
                merge_commit: false,
                ..ForgeCommit::default()
            };

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
        let analyzer_config = AnalyzerConfig::default();
        let group_parser = GroupParser::new();
        let forge_commit = ForgeCommit {
            id: "author123".into(),
            message: "feat: add feature".into(),
            author_name: "John Doe".into(),
            author_email: "john.doe@example.com".into(),
            timestamp: 1641000000,
            merge_commit: false,
            ..ForgeCommit::default()
        };
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
