use indexmap::IndexMap;

use crate::{
    analyzer::commit::Commit,
    config::versioning::{Group, Parser},
};

/// The changelog group a commit was assigned to, plus whether the
/// matching parser wants the commit dropped entirely.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    /// The value for [`Commit::group`][crate::analyzer::commit::Commit]:
    /// the group title prefixed with its `<!-- NN -->` sort tag.
    pub group: String,
    /// When `true` the commit is omitted from both the changelog and the
    /// version calculation.
    pub skip: bool,
}

impl From<&Parser> for Parsed {
    fn from(parser: &Parser) -> Self {
        let (group, skip) = parser.title_and_skip();
        Self { group, skip }
    }
}

/// Determines which changelog category a commit belongs to by matching
/// against conventional commit type patterns.
pub struct GroupParser<'a> {
    named_parsers: &'a IndexMap<Group, Parser>,
    custom_parsers: &'a [Parser],
}

impl<'a> GroupParser<'a> {
    pub fn new(
        named_parsers: &'a IndexMap<Group, Parser>,
        custom_parsers: &'a [Parser],
    ) -> Self {
        Self {
            named_parsers,
            custom_parsers,
        }
    }

    /// Determine the changelog category for a commit by checking breaking
    /// changes first, then matching commit type prefixes.
    ///
    /// Falls back to [`Group::Miscellaneous`] for anything nothing else
    /// claimed, so the returned title is never empty. Only returns `None` if
    /// `named_parsers` has no `Miscellaneous` entry at all, which
    /// [`resolve_named_parsers`][crate::resolver::resolvers::versioning] never
    /// produces.
    pub fn parse(&self, commit: &Commit) -> Option<Parsed> {
        let msg = commit.raw_message.trim();

        // custom parsers always take precedence
        for parser in self.custom_parsers.iter() {
            if parser.is_match(msg) {
                return Some(parser.into());
            }
        }

        // Handle breaking first as this one doesn't always have a pattern
        // matcher since we mostly rely on conventional commit parsing for
        // this group.
        let breaking_parser = self.named_parsers.get(&Group::Breaking);

        // If there is a user defined breaking parser i.e. pattern is some,
        // use it
        if let Some(parser) = breaking_parser
            && let Some(pattern) = parser.pattern.as_ref()
            && pattern.is_match(msg)
        {
            return Some(parser.into());
        }

        // If no user defined breaking parser is defined, use conventional
        // commit parsing to determine breaking group
        if commit.breaking
            && let Some(parser) = breaking_parser
            && parser.pattern.is_none()
        {
            return Some(parser.into());
        }

        for (group, parser) in self.named_parsers.iter() {
            if matches!(group, Group::Breaking)
                || matches!(group, Group::Miscellaneous)
            {
                // breaking already handled above and
                // miscellaneous is handled below
                continue;
            }
            if parser.is_match(msg) {
                return Some(parser.into());
            }
        }

        // Miscellaneous is the catch-all, handled last so that more
        // restrictive patterns win. Its own pattern is deliberately not
        // consulted: anything reaching this point matched nothing else, and
        // bailing out here would leave `Commit::group` empty and render a
        // blank changelog heading. Narrowing `miscellaneous.pattern` therefore
        // only changes what it claims *early*, not that it is the fallback.
        self.named_parsers
            .get(&Group::Miscellaneous)
            .map(Parsed::from)
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use crate::{
        analyzer::commit::CommitPR, config::versioning::NAMED_PARSERS,
    };

    use super::*;

    fn create_test_commit(raw_message: &str, breaking: bool) -> Commit {
        Commit {
            id: "abc123".into(),
            short_id: "abc".into(),
            group: "".into(),
            scope: None,
            title: "test message".into(),
            body: None,
            link: "https://example.com".into(),
            pr: Some(CommitPR {
                id: "22".into(),
                link: "https://example.com/pr/22".into(),
            }),
            breaking,
            breaking_description: None,
            merge_commit: false,
            timestamp: 1640995200,
            raw_title: "test message".into(),
            raw_message: raw_message.into(),
            author_name: "".into(),
            author_email: "".into(),
        }
    }

    #[test]
    fn test_group_parser_breaking_commit() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);
        let commit = create_test_commit("feat!: breaking change", true);
        let breaking_parser =
            NAMED_PARSERS.get(&Group::Breaking).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, breaking_parser.group_title());
        assert_eq!(parsed.skip, breaking_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_feat_commit() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit = create_test_commit("feat: add new feature", false);
        let feature_parser =
            NAMED_PARSERS.get(&Group::Feature).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, feature_parser.group_title());
        assert_eq!(parsed.skip, feature_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_fix_commit() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit = create_test_commit("fix: resolve bug", false);
        let fix_parser = NAMED_PARSERS.get(&Group::Fix).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, fix_parser.group_title());
        assert_eq!(parsed.skip, fix_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_chore_commit() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit = create_test_commit("chore: update dependencies", false);
        let chore_parser = NAMED_PARSERS.get(&Group::Chore).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, chore_parser.group_title());
        assert_eq!(parsed.skip, chore_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_ci_commit() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit = create_test_commit("ci: update workflow", false);
        let ci_parser = NAMED_PARSERS.get(&Group::CI).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, ci_parser.group_title());
        assert_eq!(parsed.skip, ci_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_doc_commit() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit = create_test_commit("doc: update readme", false);
        let doc_parser =
            NAMED_PARSERS.get(&Group::Documentation).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, doc_parser.group_title());
        assert_eq!(parsed.skip, doc_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_perf_commit() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit = create_test_commit("perf: optimize algorithm", false);
        let perf_parser =
            NAMED_PARSERS.get(&Group::Performance).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, perf_parser.group_title());
        assert_eq!(parsed.skip, perf_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_refactor_commit() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit = create_test_commit("refactor: clean up code", false);
        let refactor_parser =
            NAMED_PARSERS.get(&Group::Refactor).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, refactor_parser.group_title());
        assert_eq!(parsed.skip, refactor_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_revert_commit() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit = create_test_commit("revert: undo previous change", false);
        let revert_parser = NAMED_PARSERS.get(&Group::Revert).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, revert_parser.group_title());
        assert_eq!(parsed.skip, revert_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_style_commit() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit = create_test_commit("style: format code", false);
        let style_parser = NAMED_PARSERS.get(&Group::Style).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, style_parser.group_title());
        assert_eq!(parsed.skip, style_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_test_commit() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit = create_test_commit("test: add unit tests", false);
        let test_parser = NAMED_PARSERS.get(&Group::Test).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, test_parser.group_title());
        assert_eq!(parsed.skip, test_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_unknown_commit() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit = create_test_commit("random: unknown type", false);
        let misc_parser =
            NAMED_PARSERS.get(&Group::Miscellaneous).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, misc_parser.group_title());
        assert_eq!(parsed.skip, misc_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_empty_message() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit = create_test_commit("", false);
        let misc_parser =
            NAMED_PARSERS.get(&Group::Miscellaneous).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, misc_parser.group_title());
        assert_eq!(parsed.skip, misc_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_whitespace_handling() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit =
            create_test_commit("  feat: feature with leading spaces", false);
        let feature_parser =
            NAMED_PARSERS.get(&Group::Feature).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, feature_parser.group_title());
        assert_eq!(parsed.skip, feature_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_case_sensitivity() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        // Lowercase should match
        let commit1 = create_test_commit("feat: lowercase", false);
        let feature_parser =
            NAMED_PARSERS.get(&Group::Feature).cloned().unwrap();
        let parsed = parser.parse(&commit1).unwrap();
        assert_eq!(parsed.group, feature_parser.group_title());
        assert_eq!(parsed.skip, feature_parser.skip.unwrap());

        // Uppercase should not match (our regexes are case-sensitive)
        let commit2 = create_test_commit("FEAT: uppercase", false);
        let misc_parser =
            NAMED_PARSERS.get(&Group::Miscellaneous).cloned().unwrap();
        let parsed = parser.parse(&commit2).unwrap();
        assert_eq!(parsed.group, misc_parser.group_title());
        assert_eq!(parsed.skip, misc_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_breaking_takes_precedence() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        // Even if it matches feat pattern, breaking should take precedence
        let commit = create_test_commit("feat!: breaking feature", true);
        let breaking_parser =
            NAMED_PARSERS.get(&Group::Breaking).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, breaking_parser.group_title());
        assert_eq!(parsed.skip, breaking_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_with_scope() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let commit = create_test_commit("feat(api): add endpoint", false);
        let feature_parser =
            NAMED_PARSERS.get(&Group::Feature).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, feature_parser.group_title());
        assert_eq!(parsed.skip, feature_parser.skip.unwrap());
    }

    #[test]
    fn test_group_parser_multiline_message() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        let multiline_msg = "fix: resolve issue\n\nThis is a longer description\nwith multiple lines";
        let commit = create_test_commit(multiline_msg, false);
        let fix_parser = NAMED_PARSERS.get(&Group::Fix).cloned().unwrap();
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, fix_parser.group_title());
        assert_eq!(parsed.skip, fix_parser.skip.unwrap());
    }

    #[test]
    fn test_all_groups_covered() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

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
            let parsed = parser.parse(&commit).unwrap();
            let target_parser =
                NAMED_PARSERS.get(&expected_group).cloned().unwrap();
            assert_eq!(parsed.group, target_parser.group_title());
            assert_eq!(parsed.skip, target_parser.skip.unwrap());
        }
    }

    #[test]
    fn test_group_parser_order_matters() {
        let parser = GroupParser::new(&NAMED_PARSERS, &[]);

        // Breaking should always take precedence over other types
        let breaking_feat = create_test_commit(
            "feat: breaking feature\n\nBREAKING CHANGE: it broke",
            true,
        );
        let breaking_parser =
            NAMED_PARSERS.get(&Group::Breaking).cloned().unwrap();
        let breaking_title = breaking_parser.group_title();
        let breaking_skip = breaking_parser.skip.unwrap();

        let parsed = parser.parse(&breaking_feat).unwrap();
        assert_eq!(parsed.group, breaking_title);
        assert_eq!(parsed.skip, breaking_skip);

        let breaking_fix = create_test_commit("fix!: breaking fix", true);
        let parsed = parser.parse(&breaking_fix).unwrap();
        assert_eq!(parsed.group, breaking_title);
        assert_eq!(parsed.skip, breaking_skip);
    }

    #[test]
    fn test_group_parser_custom_parser_matches() {
        let custom = [Parser::new(
            Some(Regex::new(r"^deps").unwrap()),
            "📦 Deps".into(),
            false,
            0,
        )];
        let parser = GroupParser::new(&NAMED_PARSERS, &custom);

        let commit = create_test_commit("deps: bump serde", false);
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, custom[0].group_title());
        assert!(!parsed.skip);
    }

    #[test]
    fn test_group_parser_custom_parser_precedence_over_default() {
        // A custom parser whose pattern overlaps a default group (feat)
        // takes precedence over the built-in Features group.
        let custom = [Parser::new(
            Some(Regex::new(r"^feat").unwrap()),
            "Custom Features".into(),
            false,
            0,
        )];
        let parser = GroupParser::new(&NAMED_PARSERS, &custom);

        let commit = create_test_commit("feat: add thing", false);
        let parsed = parser.parse(&commit).unwrap();
        assert_eq!(parsed.group, custom[0].group_title());
    }

    #[test]
    fn test_group_parser_user_defined_breaking_pattern() {
        // Give the breaking parser an explicit pattern. This overrides
        // conventional-commit breaking detection: only messages matching
        // the pattern are classified as breaking.
        let mut named_parsers = NAMED_PARSERS.clone();
        let breaking_parser = named_parsers.get_mut(&Group::Breaking).unwrap();
        breaking_parser.pattern = Some(Regex::new(r"^breaking").unwrap());
        let breaking_title = breaking_parser.group_title();

        let parser = GroupParser::new(&named_parsers, &[]);

        // A message matching the custom pattern lands in the breaking group.
        let matching = create_test_commit("breaking: drop legacy api", false);
        let parsed = parser.parse(&matching).unwrap();
        assert_eq!(parsed.group, breaking_title);

        // A conventional `feat!:` breaking commit whose message does NOT
        // match the custom pattern falls through to the Feature group. Once
        // a breaking pattern is defined, it is the user's responsibility to
        // make it match the commits they consider breaking.
        let feat_breaking = create_test_commit("feat!: breaking feature", true);
        let feature_title =
            NAMED_PARSERS.get(&Group::Feature).unwrap().group_title();
        let parsed = parser.parse(&feat_breaking).unwrap();
        assert_eq!(parsed.group, feature_title);
    }

    /// Miscellaneous is the catch-all even when its pattern no longer matches
    /// everything. Returning no group here would leave `Commit::group` empty
    /// and render the commit under a blank `###` heading - the same failure
    /// `validate_named_parsers` rejects a blank title for.
    #[test]
    fn test_group_parser_falls_back_to_miscellaneous_when_pattern_narrowed() {
        let mut named_parsers = NAMED_PARSERS.clone();
        let misc_parser = named_parsers.get_mut(&Group::Miscellaneous).unwrap();
        misc_parser.pattern = Some(Regex::new(r"^misc").unwrap());
        let misc_title = misc_parser.group_title();

        let parser = GroupParser::new(&named_parsers, &[]);

        let commit = create_test_commit("random thing, no known prefix", false);
        let parsed = parser.parse(&commit).unwrap();

        assert_eq!(parsed.group, misc_title);
        assert!(!parsed.skip);
    }

    /// The fallback still honors the group's `skip`, so narrowing the pattern
    /// cannot smuggle commits past a `miscellaneous.skip = true`.
    #[test]
    fn test_group_parser_miscellaneous_fallback_honors_skip() {
        let mut named_parsers = NAMED_PARSERS.clone();
        let misc_parser = named_parsers.get_mut(&Group::Miscellaneous).unwrap();
        misc_parser.pattern = Some(Regex::new(r"^misc").unwrap());
        misc_parser.skip = Some(true);

        let parser = GroupParser::new(&named_parsers, &[]);

        let commit = create_test_commit("random thing, no known prefix", false);
        let parsed = parser.parse(&commit).unwrap();

        assert!(parsed.skip);
    }

    /// A hand-built parser set with no `Miscellaneous` entry is the only way
    /// `parse` yields nothing. `resolve_named_parsers` always fills every
    /// group, so this is unreachable through config.
    #[test]
    fn test_group_parser_returns_none_without_miscellaneous_parser() {
        let mut named_parsers = NAMED_PARSERS.clone();
        named_parsers.shift_remove(&Group::Miscellaneous);

        let parser = GroupParser::new(&named_parsers, &[]);

        let commit = create_test_commit("random thing, no known prefix", false);

        assert!(parser.parse(&commit).is_none());
    }
}
