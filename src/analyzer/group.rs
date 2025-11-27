use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::analyzer::commit::Commit;

/// Commit categories based on conventional commit types, used for grouping
/// changes in the changelog.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Breaking,
    Feat,
    Fix,
    Revert,
    Refactor,
    Perf,
    Doc,
    Style,
    Test,
    Chore,
    Ci,
    #[default]
    Miscellaneous,
}

impl Serialize for Group {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Group::Breaking => serializer.serialize_unit_variant(
                "Group",
                0,
                "<!-- 00 -->❌ Breaking",
            ),
            Group::Feat => serializer.serialize_unit_variant(
                "Group",
                1,
                "<!-- 01 -->🚀 Features",
            ),
            Group::Fix => serializer.serialize_unit_variant(
                "Group",
                2,
                "<!-- 02 -->🐛 Bug Fixes",
            ),
            Group::Revert => serializer.serialize_unit_variant(
                "Group",
                3,
                "<!-- 03 -->◀️ Revert",
            ),
            Group::Refactor => serializer.serialize_unit_variant(
                "Group",
                4,
                "<!-- 04 -->🚜 Refactor",
            ),
            Group::Perf => serializer.serialize_unit_variant(
                "Group",
                5,
                "<!-- 05 -->⚡ Performance",
            ),
            Group::Doc => serializer.serialize_unit_variant(
                "Group",
                6,
                "<!-- 06 -->📚 Documentation",
            ),
            Group::Style => serializer.serialize_unit_variant(
                "Group",
                7,
                "<!-- 07 -->🎨 Styling",
            ),
            Group::Test => serializer.serialize_unit_variant(
                "Group",
                8,
                "<!-- 08 -->🧪 Testing",
            ),
            Group::Chore => serializer.serialize_unit_variant(
                "Group",
                9,
                "<!-- 09 -->🧹 Chore",
            ),
            Group::Ci => serializer.serialize_unit_variant(
                "Group",
                10,
                "<!-- 10 -->⏩ CI/CD",
            ),
            Group::Miscellaneous => serializer.serialize_unit_variant(
                "Group",
                11,
                "<!-- 11 -->⚙️ Miscellaneous Tasks",
            ),
        }
    }
}

type MessageGroupParserFunction = dyn FnOnce(&Commit) -> Option<Group>;

type MessageGroupParser = Box<MessageGroupParserFunction>;

// CHORE
static CHORE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^chore").unwrap());

// CI
static CI_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^ci").unwrap());

// DOC
static DOC_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^doc").unwrap());

// FEAT
static FEAT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^feat").unwrap());

// FIX
static FIX_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^fix").unwrap());

// PERF
static PERF_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^perf").unwrap());

// REFACTOR
static REFACTOR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^refactor").unwrap());

// REVERT
static REVERT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^revert").unwrap());

// STYLE
static STYLE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^style").unwrap());

// TEST
static TEST_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^test").unwrap());

fn create_message_group_parser(
    pattern: &'static Regex,
    target_group: Group,
) -> MessageGroupParser {
    let f: MessageGroupParser = Box::new(|c: &Commit| -> Option<Group> {
        if pattern.is_match(c.raw_message.trim()) {
            return Some(target_group);
        }

        None
    });

    f
}

#[derive(Default)]
/// Determines which changelog category a commit belongs to by matching
/// against conventional commit type patterns.
pub struct GroupParser {}

impl GroupParser {
    /// Create new group parser with regex-based conventional commit type
    /// matchers.
    pub fn new() -> Self {
        Self {}
    }

    /// Determine the changelog category for a commit by checking breaking
    /// changes first, then matching commit type prefixes.
    pub fn parse(&self, commit: &Commit) -> Group {
        let parsers = self.get_parsers();
        for parser in parsers {
            if let Some(group) = parser(commit) {
                return group;
            }
        }

        Group::default()
    }

    fn get_parsers(&self) -> Vec<MessageGroupParser> {
        let breaking: MessageGroupParser = Box::new(|c: &Commit| {
            if c.breaking {
                return Some(Group::Breaking);
            }

            None
        });

        let chore = create_message_group_parser(&CHORE_REGEX, Group::Chore);
        let ci = create_message_group_parser(&CI_REGEX, Group::Ci);
        let doc = create_message_group_parser(&DOC_REGEX, Group::Doc);
        let feat = create_message_group_parser(&FEAT_REGEX, Group::Feat);
        let fix = create_message_group_parser(&FIX_REGEX, Group::Fix);
        let perf = create_message_group_parser(&PERF_REGEX, Group::Perf);
        let refactor =
            create_message_group_parser(&REFACTOR_REGEX, Group::Refactor);
        let revert = create_message_group_parser(&REVERT_REGEX, Group::Revert);
        let style = create_message_group_parser(&STYLE_REGEX, Group::Style);
        let test = create_message_group_parser(&TEST_REGEX, Group::Test);

        vec![
            breaking, feat, fix, revert, refactor, perf, doc, style, test,
            chore, ci,
        ]
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
    fn test_group_clone() {
        let group1 = Group::Feat;
        let group2 = group1.clone();
        assert_eq!(group1, group2);
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
        let parser = GroupParser::new();
        let commit = create_test_commit("feat!: breaking change", true);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Breaking);
    }

    #[test]
    fn test_group_parser_feat_commit() {
        let parser = GroupParser::new();
        let commit = create_test_commit("feat: add new feature", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Feat);
    }

    #[test]
    fn test_group_parser_fix_commit() {
        let parser = GroupParser::new();
        let commit = create_test_commit("fix: resolve bug", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Fix);
    }

    #[test]
    fn test_group_parser_chore_commit() {
        let parser = GroupParser::new();
        let commit = create_test_commit("chore: update dependencies", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Chore);
    }

    #[test]
    fn test_group_parser_ci_commit() {
        let parser = GroupParser::new();
        let commit = create_test_commit("ci: update workflow", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Ci);
    }

    #[test]
    fn test_group_parser_doc_commit() {
        let parser = GroupParser::new();
        let commit = create_test_commit("doc: update readme", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Doc);
    }

    #[test]
    fn test_group_parser_perf_commit() {
        let parser = GroupParser::new();
        let commit = create_test_commit("perf: optimize algorithm", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Perf);
    }

    #[test]
    fn test_group_parser_refactor_commit() {
        let parser = GroupParser::new();
        let commit = create_test_commit("refactor: clean up code", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Refactor);
    }

    #[test]
    fn test_group_parser_revert_commit() {
        let parser = GroupParser::new();
        let commit = create_test_commit("revert: undo previous change", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Revert);
    }

    #[test]
    fn test_group_parser_style_commit() {
        let parser = GroupParser::new();
        let commit = create_test_commit("style: format code", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Style);
    }

    #[test]
    fn test_group_parser_test_commit() {
        let parser = GroupParser::new();
        let commit = create_test_commit("test: add unit tests", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Test);
    }

    #[test]
    fn test_group_parser_unknown_commit() {
        let parser = GroupParser::new();
        let commit = create_test_commit("random: unknown type", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Miscellaneous);
    }

    #[test]
    fn test_group_parser_empty_message() {
        let parser = GroupParser::new();
        let commit = create_test_commit("", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Miscellaneous);
    }

    #[test]
    fn test_group_parser_whitespace_handling() {
        let parser = GroupParser::new();
        let commit =
            create_test_commit("  feat: feature with leading spaces", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Feat);
    }

    #[test]
    fn test_group_parser_case_sensitivity() {
        let parser = GroupParser::new();

        // Lowercase should match
        let commit1 = create_test_commit("feat: lowercase", false);
        assert_eq!(parser.parse(&commit1), Group::Feat);

        // Uppercase should not match (our regexes are case-sensitive)
        let commit2 = create_test_commit("FEAT: uppercase", false);
        assert_eq!(parser.parse(&commit2), Group::Miscellaneous);
    }

    #[test]
    fn test_group_parser_breaking_takes_precedence() {
        let parser = GroupParser::new();
        // Even if it matches feat pattern, breaking should take precedence
        let commit = create_test_commit("feat!: breaking feature", true);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Breaking);
    }

    #[test]
    fn test_group_parser_with_scope() {
        let parser = GroupParser::new();
        let commit = create_test_commit("feat(api): add endpoint", false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Feat);
    }

    #[test]
    fn test_group_parser_multiline_message() {
        let parser = GroupParser::new();
        let multiline_msg = "fix: resolve issue\n\nThis is a longer description\nwith multiple lines";
        let commit = create_test_commit(multiline_msg, false);
        let group = parser.parse(&commit);
        assert_eq!(group, Group::Fix);
    }

    #[test]
    fn test_regex_patterns() {
        // Test the regex patterns directly
        assert!(FEAT_REGEX.is_match("feat: something"));
        assert!(FEAT_REGEX.is_match("feat(scope): something"));
        assert!(FEAT_REGEX.is_match("feature: something")); // "feature" starts with "feat"

        assert!(FIX_REGEX.is_match("fix: something"));
        assert!(FIX_REGEX.is_match("fix(bug): something"));
        assert!(FIX_REGEX.is_match("fixed: something")); // "fixed" starts with "fix"

        assert!(CHORE_REGEX.is_match("chore: something"));
        assert!(CHORE_REGEX.is_match("choreography: something")); // "choreography" starts with "chore"

        assert!(DOC_REGEX.is_match("doc: something"));
        assert!(DOC_REGEX.is_match("docs: something")); // "docs" starts with "doc"
    }

    #[test]
    fn test_all_groups_covered() {
        let parser = GroupParser::new();

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
        let parser = GroupParser::new();

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
