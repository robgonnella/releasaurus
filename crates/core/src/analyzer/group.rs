use indexmap::IndexMap;

use crate::{
    analyzer::commit::Commit,
    config::changelog::{Group, Parser},
};

/// Determines which changelog category a commit belongs to by matching
/// against conventional commit type patterns.
pub struct GroupParser<'a> {
    default_parsers: &'a IndexMap<Group, Parser>,
    custom_parsers: &'a [Parser],
}

impl<'a> GroupParser<'a> {
    pub fn new(
        default_parsers: &'a IndexMap<Group, Parser>,
        custom_parsers: &'a [Parser],
    ) -> Self {
        Self {
            default_parsers,
            custom_parsers,
        }
    }

    /// Determine the changelog category for a commit by checking breaking
    /// changes first, then matching commit type prefixes.
    pub fn parse(&self, commit: &Commit) -> Option<(String, bool)> {
        let msg = commit.raw_message.trim();

        // custom parsers always take precedence
        for parser in self.custom_parsers.iter() {
            if parser.is_match(msg) {
                return Some(parser.title_and_skip());
            }
        }

        // Handle breaking first as this one doesn't always have a pattern
        // matcher since we mostly rely on conventional commit parsing for
        // this group.
        let breaking_parser = self.default_parsers.get(&Group::Breaking);

        // If there is a user defined breaking parser i.e. pattern is some,
        // use it
        if let Some(parser) = breaking_parser
            && let Some(pattern) = parser.pattern.as_ref()
            && pattern.is_match(msg)
        {
            return Some(parser.title_and_skip());
        }

        // If no user defined breaking parser is defined, use conventional
        // commit parsing to determine breaking group
        if commit.breaking
            && let Some(parser) = self.default_parsers.get(&Group::Breaking)
            && parser.pattern.is_none()
        {
            return Some(parser.title_and_skip());
        }

        for (group, parser) in self.default_parsers.iter() {
            if matches!(group, Group::Breaking)
                || matches!(group, Group::Miscellaneous)
            {
                // breaking already handled above and
                // miscellaneous is handled below
                continue;
            }
            if parser.is_match(msg) {
                return Some(parser.title_and_skip());
            }
        }

        // Always handle miscellaneous group last as its regex pattern
        // is likely .* and we want to give precedence to other more
        // restrictive patterns.
        if let Some(parser) = self.default_parsers.get(&Group::Miscellaneous)
            && parser.is_match(msg)
        {
            return Some(parser.title_and_skip());
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use crate::config::changelog::DEFAULT_PARSERS;

    use super::*;

    fn create_test_commit(raw_message: &str, breaking: bool) -> Commit {
        Commit {
            id: "abc123".to_string(),
            short_id: "abc".to_string(),
            group: "".into(),
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
    fn test_group_parser_breaking_commit() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);
        let commit = create_test_commit("feat!: breaking change", true);
        let breaking_parser =
            DEFAULT_PARSERS.get(&Group::Breaking).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, breaking_parser.title.unwrap());
        assert_eq!(skip, breaking_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_feat_commit() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit = create_test_commit("feat: add new feature", false);
        let feature_parser =
            DEFAULT_PARSERS.get(&Group::Feature).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, feature_parser.title.unwrap());
        assert_eq!(skip, feature_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_fix_commit() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit = create_test_commit("fix: resolve bug", false);
        let fix_parser = DEFAULT_PARSERS.get(&Group::Fix).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, fix_parser.title.unwrap());
        assert_eq!(skip, fix_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_chore_commit() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit = create_test_commit("chore: update dependencies", false);
        let chore_parser = DEFAULT_PARSERS.get(&Group::Chore).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, chore_parser.title.unwrap());
        assert_eq!(skip, chore_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_ci_commit() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit = create_test_commit("ci: update workflow", false);
        let ci_parser = DEFAULT_PARSERS.get(&Group::CI).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, ci_parser.title.unwrap());
        assert_eq!(skip, ci_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_doc_commit() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit = create_test_commit("doc: update readme", false);
        let doc_parser =
            DEFAULT_PARSERS.get(&Group::Documentation).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, doc_parser.title.unwrap());
        assert_eq!(skip, doc_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_perf_commit() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit = create_test_commit("perf: optimize algorithm", false);
        let perf_parser =
            DEFAULT_PARSERS.get(&Group::Performance).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, perf_parser.title.unwrap());
        assert_eq!(skip, perf_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_refactor_commit() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit = create_test_commit("refactor: clean up code", false);
        let refactor_parser =
            DEFAULT_PARSERS.get(&Group::Refactor).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, refactor_parser.title.unwrap());
        assert_eq!(skip, refactor_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_revert_commit() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit = create_test_commit("revert: undo previous change", false);
        let revert_parser =
            DEFAULT_PARSERS.get(&Group::Revert).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, revert_parser.title.unwrap());
        assert_eq!(skip, revert_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_style_commit() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit = create_test_commit("style: format code", false);
        let style_parser = DEFAULT_PARSERS.get(&Group::Style).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, style_parser.title.unwrap());
        assert_eq!(skip, style_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_test_commit() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit = create_test_commit("test: add unit tests", false);
        let test_parser = DEFAULT_PARSERS.get(&Group::Test).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, test_parser.title.unwrap());
        assert_eq!(skip, test_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_unknown_commit() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit = create_test_commit("random: unknown type", false);
        let misc_parser =
            DEFAULT_PARSERS.get(&Group::Miscellaneous).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, misc_parser.title.unwrap());
        assert_eq!(skip, misc_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_empty_message() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit = create_test_commit("", false);
        let misc_parser =
            DEFAULT_PARSERS.get(&Group::Miscellaneous).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, misc_parser.title.unwrap());
        assert_eq!(skip, misc_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_whitespace_handling() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit =
            create_test_commit("  feat: feature with leading spaces", false);
        let feature_parser =
            DEFAULT_PARSERS.get(&Group::Feature).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, feature_parser.title.unwrap());
        assert_eq!(skip, feature_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_case_sensitivity() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        // Lowercase should match
        let commit1 = create_test_commit("feat: lowercase", false);
        let feature_parser =
            DEFAULT_PARSERS.get(&Group::Feature).cloned().unwrap();
        let (title, skip) = parser.parse(&commit1).unwrap();
        assert_eq!(title, feature_parser.title.unwrap());
        assert_eq!(skip, feature_parser.skip.unwrap());

        // Uppercase should not match (our regexes are case-sensitive)
        let commit2 = create_test_commit("FEAT: uppercase", false);
        let misc_parser =
            DEFAULT_PARSERS.get(&Group::Miscellaneous).cloned().unwrap();
        let (title, skip) = parser.parse(&commit2).unwrap();
        assert_eq!(title, misc_parser.title.unwrap());
        assert_eq!(skip, misc_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_breaking_takes_precedence() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        // Even if it matches feat pattern, breaking should take precedence
        let commit = create_test_commit("feat!: breaking feature", true);
        let breaking_parser =
            DEFAULT_PARSERS.get(&Group::Breaking).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, breaking_parser.title.unwrap());
        assert_eq!(skip, breaking_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_with_scope() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let commit = create_test_commit("feat(api): add endpoint", false);
        let feature_parser =
            DEFAULT_PARSERS.get(&Group::Feature).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, feature_parser.title.unwrap());
        assert_eq!(skip, feature_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_multiline_message() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        let multiline_msg = "fix: resolve issue\n\nThis is a longer description\nwith multiple lines";
        let commit = create_test_commit(multiline_msg, false);
        let fix_parser = DEFAULT_PARSERS.get(&Group::Fix).cloned().unwrap();
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, fix_parser.title.unwrap());
        assert_eq!(skip, fix_parser.skip.unwrap());
    }

    #[test]
    fn test_all_groups_covered() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        // Test that we have parsers for all the main groups
        let test_cases = vec![
            ("feat: test", Group::Feature),
            ("fix: test", Group::Fix),
            ("chore: test", Group::Chore),
            ("doc: test", Group::Documentation),
            ("style: test", Group::Style),
            ("refactor: test", Group::Refactor),
            ("perf: test", Group::Performance),
            ("test: test", Group::Test),
            ("revert: test", Group::Revert),
            ("ci: test", Group::CI),
        ];

        for (message, expected_group) in test_cases {
            let commit = create_test_commit(message, false);
            let (title, skip) = parser.parse(&commit).unwrap();
            let target_parser =
                DEFAULT_PARSERS.get(&expected_group).cloned().unwrap();
            assert_eq!(title, target_parser.title.unwrap());
            assert_eq!(skip, target_parser.skip.unwrap());
        }
    }

    #[test]
    fn test_group_parser_order_matters() {
        let parser = GroupParser::new(&DEFAULT_PARSERS, &[]);

        // Breaking should always take precedence over other types
        let breaking_feat = create_test_commit(
            "feat: breaking feature\n\nBREAKING CHANGE: it broke",
            true,
        );
        let breaking_parser =
            DEFAULT_PARSERS.get(&Group::Breaking).cloned().unwrap();
        let breaking_title = breaking_parser.title.unwrap();
        let breaking_skip = breaking_parser.skip.unwrap();

        let (title, skip) = parser.parse(&breaking_feat).unwrap();
        assert_eq!(title, breaking_title);
        assert_eq!(skip, breaking_skip);

        let breaking_fix = create_test_commit("fix!: breaking fix", true);
        let (title, skip) = parser.parse(&breaking_fix).unwrap();
        assert_eq!(title, breaking_title);
        assert_eq!(skip, breaking_skip);
    }

    #[test]
    fn test_group_parser_custom_parser_matches() {
        let custom = [Parser::new(
            Some(Regex::new(r"^deps").unwrap()),
            "📦 Deps".into(),
            false,
        )];
        let parser = GroupParser::new(&DEFAULT_PARSERS, &custom);

        let commit = create_test_commit("deps: bump serde", false);
        let (title, skip) = parser.parse(&commit).unwrap();
        assert_eq!(title, "📦 Deps");
        assert!(!skip);
    }

    #[test]
    fn test_group_parser_custom_parser_precedence_over_default() {
        // A custom parser whose pattern overlaps a default group (feat)
        // takes precedence over the built-in Features group.
        let custom = [Parser::new(
            Some(Regex::new(r"^feat").unwrap()),
            "Custom Features".into(),
            false,
        )];
        let parser = GroupParser::new(&DEFAULT_PARSERS, &custom);

        let commit = create_test_commit("feat: add thing", false);
        let (title, _) = parser.parse(&commit).unwrap();
        assert_eq!(title, "Custom Features");
    }

    #[test]
    fn test_group_parser_user_defined_breaking_pattern() {
        // Give the breaking parser an explicit pattern. This overrides
        // conventional-commit breaking detection: only messages matching
        // the pattern are classified as breaking.
        let mut default_parsers = DEFAULT_PARSERS.clone();
        let breaking_parser =
            default_parsers.get_mut(&Group::Breaking).unwrap();
        breaking_parser.pattern = Some(Regex::new(r"^breaking").unwrap());
        let breaking_title = breaking_parser.title.clone().unwrap();

        let parser = GroupParser::new(&default_parsers, &[]);

        // A message matching the custom pattern lands in the breaking group.
        let matching = create_test_commit("breaking: drop legacy api", false);
        let (title, _) = parser.parse(&matching).unwrap();
        assert_eq!(title, breaking_title);

        // A conventional `feat!:` breaking commit whose message does NOT
        // match the custom pattern falls through to the Feature group. Once
        // a breaking pattern is defined, it is the user's responsibility to
        // make it match the commits they consider breaking.
        let feat_breaking = create_test_commit("feat!: breaking feature", true);
        let feature_title = DEFAULT_PARSERS
            .get(&Group::Feature)
            .unwrap()
            .title
            .clone()
            .unwrap();
        let (title, _) = parser.parse(&feat_breaking).unwrap();
        assert_eq!(title, feature_title);
    }
}
