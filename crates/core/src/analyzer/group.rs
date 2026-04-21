use std::fmt;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::analyzer::commit::Commit;

/// Commit categories based on conventional commit types, used for grouping
/// changes in the changelog.
#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
pub enum Group {
    #[serde(rename = "<!-- 00 -->❌ Breaking")]
    Breaking,
    #[serde(rename = "<!-- 01 -->🚀 Features")]
    Feat,
    #[serde(rename = "<!-- 02 -->🐛 Bug Fixes")]
    Fix,
    #[serde(rename = "<!-- 03 -->◀️ Revert")]
    Revert,
    #[serde(rename = "<!-- 04 -->🚜 Refactor")]
    Refactor,
    #[serde(rename = "<!-- 05 -->⚡ Performance")]
    Perf,
    #[serde(rename = "<!-- 06 -->📚 Documentation")]
    Doc,
    #[serde(rename = "<!-- 07 -->🎨 Styling")]
    Style,
    #[serde(rename = "<!-- 08 -->🧪 Testing")]
    Test,
    #[serde(rename = "<!-- 09 -->🧹 Chore")]
    Chore,
    #[serde(rename = "<!-- 10 -->⏩ CI/CD")]
    Ci,
    #[serde(rename = "<!-- 11 -->⚙️ Miscellaneous Tasks")]
    #[default]
    Miscellaneous,
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Group::Breaking => "breaking",
            Group::Feat => "feat",
            Group::Fix => "fix",
            Group::Revert => "revert",
            Group::Refactor => "refactor",
            Group::Perf => "perf",
            Group::Doc => "doc",
            Group::Style => "style",
            Group::Test => "test",
            Group::Chore => "chore",
            Group::Ci => "ci",
            Group::Miscellaneous => "miscellaneous",
        };
        write!(f, "{s}")
    }
}

// Parser data structure that can parse groups from commit message patterns
struct Parser {
    pattern: Regex,
    target_group: Group,
}

impl Parser {
    fn new(pattern: Regex, target_group: Group) -> Self {
        Self {
            pattern,
            target_group,
        }
    }

    pub fn parse(&self, c: &Commit) -> Option<Group> {
        if self.pattern.is_match(c.raw_message.trim()) {
            return Some(self.target_group);
        }

        None
    }
}

// CHORE
static CHORE_PARSER: LazyLock<Parser> =
    LazyLock::new(|| Parser::new(Regex::new(r"^chore").unwrap(), Group::Chore));

// CI
static CI_PARSER: LazyLock<Parser> =
    LazyLock::new(|| Parser::new(Regex::new(r"^ci").unwrap(), Group::Ci));

// DOC
static DOC_PARSER: LazyLock<Parser> =
    LazyLock::new(|| Parser::new(Regex::new(r"^doc").unwrap(), Group::Doc));

// FEAT
static FEAT_PARSER: LazyLock<Parser> =
    LazyLock::new(|| Parser::new(Regex::new(r"^feat").unwrap(), Group::Feat));

// FIX
static FIX_PARSER: LazyLock<Parser> =
    LazyLock::new(|| Parser::new(Regex::new(r"^fix").unwrap(), Group::Fix));

// PERF
static PERF_PARSER: LazyLock<Parser> =
    LazyLock::new(|| Parser::new(Regex::new(r"^perf").unwrap(), Group::Perf));

// REFACTOR
static REFACTOR_PARSER: LazyLock<Parser> = LazyLock::new(|| {
    Parser::new(Regex::new(r"^refactor").unwrap(), Group::Refactor)
});

// REVERT
static REVERT_PARSER: LazyLock<Parser> = LazyLock::new(|| {
    Parser::new(Regex::new(r"^revert").unwrap(), Group::Revert)
});

// STYLE
static STYLE_PARSER: LazyLock<Parser> =
    LazyLock::new(|| Parser::new(Regex::new(r"^style").unwrap(), Group::Style));

// TEST
static TEST_PARSER: LazyLock<Parser> =
    LazyLock::new(|| Parser::new(Regex::new(r"^test").unwrap(), Group::Test));

static GROUP_PARSERS: [&LazyLock<Parser>; 10] = [
    &FEAT_PARSER,
    &FIX_PARSER,
    &REVERT_PARSER,
    &REFACTOR_PARSER,
    &PERF_PARSER,
    &DOC_PARSER,
    &STYLE_PARSER,
    &TEST_PARSER,
    &CHORE_PARSER,
    &CI_PARSER,
];

/// Determines which changelog category a commit belongs to by matching
/// against conventional commit type patterns.
#[derive(Default)]
pub struct GroupParser {}

impl GroupParser {
    /// Determine the changelog category for a commit by checking breaking
    /// changes first, then matching commit type prefixes.
    pub fn parse(&self, commit: &Commit) -> Group {
        if commit.breaking {
            return Group::Breaking;
        }

        for parser in GROUP_PARSERS {
            if let Some(group) = parser.parse(commit) {
                return group;
            }
        }

        Group::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_commit(raw_message: &str, breaking: bool) -> Commit {
        Commit {
            id: "abc123".to_string(),
            short_id: "abc".to_string(),
            group: Group::default(),
            scope: None,
            title: "test message".to_string(),
            body: None,
            link: "https://example.com".to_string(),
            breaking,
            breaking_description: None,
            merge_commit: false,
            timestamp: 1640995200,
            raw_title: "test message".to_string(),
            raw_message: raw_message.to_string(),
            author_name: "".into(),
            author_email: "".into(),
        }
    }

    #[test]
    fn test_group_default() {
        let group = Group::default();
        assert_eq!(group, Group::Miscellaneous);
    }

    #[test]
    fn test_group_equality() {
        assert_eq!(Group::Feat, Group::Feat);
        assert_eq!(Group::Fix, Group::Fix);
        assert_eq!(Group::Breaking, Group::Breaking);
        assert_ne!(Group::Feat, Group::Fix);
        assert_ne!(Group::Breaking, Group::Miscellaneous);
    }

    #[test]
    fn test_group_ordering() {
        // Test that Breaking comes first in sort order
        let mut groups = [Group::Fix, Group::Breaking, Group::Feat];
        groups.sort();
        assert_eq!(groups[0], Group::Breaking);

        // Test other orderings
        assert!(Group::Breaking < Group::Feat);
        assert!(Group::Feat < Group::Fix);
        assert!(Group::Miscellaneous > Group::Ci); // Other should be last
    }

    #[test]
    fn test_group_serialization() {
        let test_cases = vec![
            (Group::Breaking, "❌ Breaking"),
            (Group::Feat, "🚀 Features"),
            (Group::Fix, "🐛 Bug Fixes"),
            (Group::Revert, "◀️ Revert"),
            (Group::Refactor, "🚜 Refactor"),
            (Group::Perf, "⚡ Performance"),
            (Group::Doc, "📚 Documentation"),
            (Group::Style, "🎨 Styling"),
            (Group::Test, "🧪 Testing"),
            (Group::Chore, "🧹 Chore"),
            (Group::Ci, "⏩ CI/CD"),
            (Group::Miscellaneous, "⚙️ Miscellaneous Tasks"),
        ];

        for (group, expected) in test_cases {
            let json = serde_json::to_string(&group)
                .expect("Failed to serialize group");
            assert!(
                json.contains(expected),
                "Group {:?} should serialize to contain '{}'",
                group,
                expected
            );
        }
    }

    #[test]
    fn test_group_parser_breaking_commit() {
        let parser = GroupParser::default();
        let commit = create_test_commit("feat!: breaking change", true);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Breaking);
    }

    #[test]
    fn test_group_parser_feat_commit() {
        let parser = GroupParser::default();
        let commit = create_test_commit("feat: add new feature", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Feat);
    }

    #[test]
    fn test_group_parser_fix_commit() {
        let parser = GroupParser::default();
        let commit = create_test_commit("fix: resolve bug", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Fix);
    }

    #[test]
    fn test_group_parser_chore_commit() {
        let parser = GroupParser::default();
        let commit = create_test_commit("chore: update dependencies", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Chore);
    }

    #[test]
    fn test_group_parser_ci_commit() {
        let parser = GroupParser::default();
        let commit = create_test_commit("ci: update workflow", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Ci);
    }

    #[test]
    fn test_group_parser_doc_commit() {
        let parser = GroupParser::default();
        let commit = create_test_commit("doc: update readme", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Doc);
    }

    #[test]
    fn test_group_parser_perf_commit() {
        let parser = GroupParser::default();
        let commit = create_test_commit("perf: optimize algorithm", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Perf);
    }

    #[test]
    fn test_group_parser_refactor_commit() {
        let parser = GroupParser::default();
        let commit = create_test_commit("refactor: clean up code", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Refactor);
    }

    #[test]
    fn test_group_parser_revert_commit() {
        let parser = GroupParser::default();
        let commit = create_test_commit("revert: undo previous change", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Revert);
    }

    #[test]
    fn test_group_parser_style_commit() {
        let parser = GroupParser::default();
        let commit = create_test_commit("style: format code", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Style);
    }

    #[test]
    fn test_group_parser_test_commit() {
        let parser = GroupParser::default();
        let commit = create_test_commit("test: add unit tests", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Test);
    }

    #[test]
    fn test_group_parser_unknown_commit() {
        let parser = GroupParser::default();
        let commit = create_test_commit("random: unknown type", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Miscellaneous);
    }

    #[test]
    fn test_group_parser_empty_message() {
        let parser = GroupParser::default();
        let commit = create_test_commit("", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Miscellaneous);
    }

    #[test]
    fn test_group_parser_whitespace_handling() {
        let parser = GroupParser::default();
        let commit =
            create_test_commit("  feat: feature with leading spaces", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Feat);
    }

    #[test]
    fn test_group_parser_case_sensitivity() {
        let parser = GroupParser::default();

        // Lowercase should match
        let commit1 = create_test_commit("feat: lowercase", false);
        assert_eq!(parser.parse(&commit1), Group::Feat);

        // Uppercase should not match (our regexes are case-sensitive)
        let commit2 = create_test_commit("FEAT: uppercase", false);
        assert_eq!(parser.parse(&commit2), Group::Miscellaneous);
    }

    #[test]
    fn test_group_parser_breaking_takes_precedence() {
        let parser = GroupParser::default();
        // Even if it matches feat pattern, breaking should take precedence
        let commit = create_test_commit("feat!: breaking feature", true);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Breaking);
    }

    #[test]
    fn test_group_parser_with_scope() {
        let parser = GroupParser::default();
        let commit = create_test_commit("feat(api): add endpoint", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Feat);
    }

    #[test]
    fn test_group_parser_multiline_message() {
        let parser = GroupParser::default();
        let multiline_msg = "fix: resolve issue\n\nThis is a longer description\nwith multiple lines";
        let commit = create_test_commit(multiline_msg, false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Fix);
    }

    #[test]
    fn test_all_groups_covered() {
        let parser = GroupParser::default();

        // Test that we have parsers for all the main groups
        let test_cases = vec![
            ("feat: test", Group::Feat),
            ("fix: test", Group::Fix),
            ("chore: test", Group::Chore),
            ("doc: test", Group::Doc),
            ("style: test", Group::Style),
            ("refactor: test", Group::Refactor),
            ("perf: test", Group::Perf),
            ("test: test", Group::Test),
            ("revert: test", Group::Revert),
            ("ci: test", Group::Ci),
        ];

        for (message, expected_group) in test_cases {
            let commit = create_test_commit(message, false);
            let parsed_group = parser.parse(&commit);
            assert_eq!(
                parsed_group, expected_group,
                "Failed for message: {}",
                message
            );
        }
    }

    #[test]
    fn test_group_parser_order_matters() {
        let parser = GroupParser::default();

        // Breaking should always take precedence over other types
        let breaking_feat = create_test_commit(
            "feat: breaking feature\n\nBREAKING CHANGE: it broke",
            true,
        );
        assert_eq!(parser.parse(&breaking_feat), Group::Breaking);

        let breaking_fix = create_test_commit("fix!: breaking fix", true);
        assert_eq!(parser.parse(&breaking_fix), Group::Breaking);
    }
}
