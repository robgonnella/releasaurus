use std::sync::LazyLock;

use derive_builder::Builder;
use indexmap::IndexMap;
use merge::Merge;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::config::prerelease::PrereleaseConfig;

pub const DEFAULT_SKIP_MERGE_COMMITS: bool = true;
pub const DEFAULT_BREAKING_ALWAYS_INCREMENT_MAJOR: bool = true;
pub const DEFAULT_FEAT_ALWAYS_INCREMENT_MINOR: bool = true;

/// Commit categories based on conventional commit types, used for grouping
/// changes in the changelog.
#[derive(
    Debug,
    Copy,
    Clone,
    Display,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    JsonSchema,
    Hash,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Group {
    Breaking,
    Feature,
    Fix,
    Revert,
    Refactor,
    Performance,
    Documentation,
    Style,
    Test,
    Chore,
    CI,
    Miscellaneous,
}

/// Highest accepted [`Parser::order`].
///
/// Order is rendered into a fixed two-digit sort tag, so lexicographic
/// ordering only matches numeric ordering while every value is two digits
/// wide (`"99"` would otherwise sort after `"100"`).
pub const MAX_PARSER_ORDER: u8 = 99;

#[derive(Debug, Default, Clone, Serialize, Deserialize, Merge, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parser {
    #[schemars(with = "String")]
    #[serde(default, with = "serde_regex")]
    #[merge(strategy = merge::option::overwrite_none)]
    pub pattern: Option<Regex>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub title: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub skip: Option<bool>,
    /// Position of this group in the changelog, `0`-`99`, lowest first.
    /// Groups sharing an order are ordered by title.
    #[merge(strategy = merge::option::overwrite_none)]
    #[schemars(range(min = 0, max = 99))]
    pub order: Option<u8>,
}

impl Parser {
    pub fn new(
        pattern: Option<Regex>,
        title: String,
        skip: bool,
        order: u8,
    ) -> Self {
        Self {
            pattern,
            title: Some(title),
            skip: Some(skip),
            order: Some(order),
        }
    }

    pub fn is_match(&self, msg: &str) -> bool {
        self.pattern.as_ref().is_some_and(|p| p.is_match(msg))
    }

    /// The value stored on [`Commit::group`][crate::analyzer::commit::Commit],
    /// which is both the changelog heading and its sort key: the title
    /// prefixed with a `<!-- NN -->` tag built from [`Self::order`].
    ///
    /// The default template sorts on this string and strips the tag before
    /// rendering, so `order` controls heading order without appearing in the
    /// changelog. Resolution and validation guarantee an `order` is present;
    /// the fallback only keeps an unvalidated parser from claiming first
    /// position.
    pub fn group_title(&self) -> String {
        format!(
            "<!-- {:02} -->{}",
            self.order.unwrap_or(MAX_PARSER_ORDER),
            self.title.as_deref().unwrap_or_default()
        )
    }

    pub fn title_and_skip(&self) -> (String, bool) {
        (self.group_title(), self.skip.unwrap_or_default())
    }
}

fn default_breaking_always_increment_major() -> bool {
    DEFAULT_BREAKING_ALWAYS_INCREMENT_MAJOR
}

fn default_features_always_increment_minor() -> bool {
    DEFAULT_FEAT_ALWAYS_INCREMENT_MINOR
}

fn default_skip_merge_commits() -> bool {
    DEFAULT_SKIP_MERGE_COMMITS
}

fn default_named_parsers() -> IndexMap<Group, Parser> {
    NAMED_PARSERS.clone()
}

pub static NAMED_PARSERS: LazyLock<IndexMap<Group, Parser>> =
    LazyLock::new(|| {
        let chore_regex = Regex::new(r"^chore").unwrap();
        let ci_regex = Regex::new(r"^ci").unwrap();
        let doc_regex = Regex::new(r"^doc").unwrap();
        let feat_regex = Regex::new(r"^feat").unwrap();
        let fix_regex = Regex::new(r"^fix").unwrap();
        let perf_regex = Regex::new(r"^perf").unwrap();
        let refactor_regex = Regex::new(r"^refactor").unwrap();
        let revert_regex = Regex::new(r"^revert").unwrap();
        let style_regex = Regex::new(r"^style").unwrap();
        let test_regex = Regex::new(r"^test").unwrap();
        let misc_regex = Regex::new(r".*").unwrap();
        IndexMap::from([
            (
                Group::Breaking,
                Parser::new(None, "❌ Breaking".into(), false, 0),
            ),
            (
                Group::Feature,
                Parser::new(Some(feat_regex), "🚀 Features".into(), false, 1),
            ),
            (
                Group::Fix,
                Parser::new(Some(fix_regex), "🐛 Bug Fixes".into(), false, 2),
            ),
            (
                Group::Revert,
                Parser::new(Some(revert_regex), "◀️ Revert".into(), false, 3),
            ),
            (
                Group::Refactor,
                Parser::new(
                    Some(refactor_regex),
                    "🚜 Refactor".into(),
                    false,
                    4,
                ),
            ),
            (
                Group::Performance,
                Parser::new(
                    Some(perf_regex),
                    "⚡ Performance".into(),
                    false,
                    5,
                ),
            ),
            (
                Group::Documentation,
                Parser::new(
                    Some(doc_regex),
                    "📚 Documentation".into(),
                    false,
                    6,
                ),
            ),
            (
                Group::Style,
                Parser::new(Some(style_regex), "🎨 Styling".into(), false, 7),
            ),
            (
                Group::Test,
                Parser::new(Some(test_regex), "🧪 Testing".into(), false, 8),
            ),
            (
                Group::Chore,
                Parser::new(Some(chore_regex), "🧹 Chore".into(), false, 9),
            ),
            (
                Group::CI,
                Parser::new(Some(ci_regex), "⏩ CI/CD".into(), false, 10),
            ),
            (
                Group::Miscellaneous,
                Parser::new(
                    Some(misc_regex),
                    "⚙️ Miscellaneous Tasks".into(),
                    false,
                    11,
                ),
            ),
        ])
    });

#[derive(Debug, Default, Clone, Serialize, Deserialize, Merge, JsonSchema)]
pub struct ParserList(#[merge(strategy = merge::vec::append)] pub Vec<Parser>);

/// Versioning config for calculating the next release version
#[derive(
    Debug, Default, Clone, Serialize, Deserialize, Merge, JsonSchema, Builder,
)]
#[builder(setter(into, strip_option), default)]
#[serde(deny_unknown_fields)]
pub struct VersioningConfig {
    /// Prerelease configuration (suffix + strategy)
    #[merge(strategy = merge::option::overwrite_none)]
    pub prerelease: Option<PrereleaseConfig>,
    /// Auto-starts next release by creating a release PR with a patch version
    /// bump immediately after creating a release
    #[merge(strategy = merge::option::overwrite_none)]
    pub auto_start_next: Option<bool>,
    /// Always increments major version on breaking commits
    #[merge(strategy = merge::option::overwrite_none)]
    #[schemars(default = "default_breaking_always_increment_major")]
    pub breaking_always_increment_major: Option<bool>,
    /// Always increments minor version on feature commits
    #[merge(strategy = merge::option::overwrite_none)]
    #[schemars(default = "default_features_always_increment_minor")]
    pub features_always_increment_minor: Option<bool>,
    /// Custom regex pattern matched against commit messages to trigger a
    /// major version bump. This is additive — breaking change commits always
    /// trigger major bumps regardless of this setting. In TOML double-quoted
    /// strings, escape backslashes (e.g. `"\\[BREAKING\\]"` matches
    /// `[BREAKING]`).
    #[merge(strategy = merge::option::overwrite_none)]
    pub custom_major_increment_regex: Option<String>,
    /// Custom regex pattern matched against commit messages to trigger a
    /// minor version bump. This is additive — `feat:` commits always trigger
    /// minor bumps regardless of this setting. In TOML double-quoted strings,
    /// escape backslashes (e.g. `"\\[FEATURE\\]"` matches `[FEATURE]`).
    #[merge(strategy = merge::option::overwrite_none)]
    pub custom_minor_increment_regex: Option<String>,
    /// Skips including merge commits in changelog
    #[merge(strategy = merge::option::overwrite_none)]
    #[schemars(default = "default_skip_merge_commits")]
    pub skip_merge_commits: Option<bool>,
    /// Named parsers for organizing commits into common groups e.g. feature,
    /// bug, etc. These can be turned off by setting the "skip" field to "true".
    /// When skipped, these commit types (groups) will not result in version
    /// bumps. Use the "order" field to position a group in the changelog:
    /// for example to show bug fixes before features, set fix order = 1 and
    /// feature order = 2. Anything defined in this section will be merged
    /// with, and override, the pre-defined default parsers. So, for example,
    /// to only skip just CI commits, you only need to define the "ci" parser
    /// and set the "skip" field to true. All other parsers will remain as
    /// default.
    #[merge(skip)]
    #[schemars(default = "default_named_parsers")]
    pub named_parsers: Option<IndexMap<Group, Parser>>,
    /// Additional parsers for grouping commits into non-default groups
    /// e.g. pattern="^special:" title="Special" order=0 skip=false.
    /// Unlike named parsers these have no defaults to fall back on, so
    /// "pattern", "title" and "order" are all required.
    #[merge(strategy = merge::option::recurse)]
    #[serde(rename = "custom_parser")]
    pub custom_parsers: Option<ParserList>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_equality() {
        assert_eq!(Group::Feature, Group::Feature);
        assert_eq!(Group::Fix, Group::Fix);
        assert_eq!(Group::Breaking, Group::Breaking);
        assert_ne!(Group::Feature, Group::Fix);
        assert_ne!(Group::Breaking, Group::Miscellaneous);
    }

    #[test]
    fn test_group_ordering() {
        // Test that Breaking comes first in sort order
        let mut groups = [Group::Fix, Group::Breaking, Group::Feature];
        groups.sort();
        assert_eq!(groups[0], Group::Breaking);

        // Test other orderings
        assert!(Group::Breaking < Group::Feature);
        assert!(Group::Feature < Group::Fix);
        assert!(Group::Miscellaneous > Group::CI); // Other should be last
    }

    #[test]
    fn test_group_serialization() {
        let test_cases = vec![
            (Group::Breaking, "breaking"),
            (Group::Feature, "feature"),
            (Group::Fix, "fix"),
            (Group::Revert, "revert"),
            (Group::Refactor, "refactor"),
            (Group::Performance, "performance"),
            (Group::Documentation, "documentation"),
            (Group::Style, "style"),
            (Group::Test, "test"),
            (Group::Chore, "chore"),
            (Group::CI, "ci"),
            (Group::Miscellaneous, "miscellaneous"),
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
}
