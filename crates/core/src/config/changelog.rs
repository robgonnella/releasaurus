use derive_builder::Builder;
use indexmap::IndexMap;
use merge::Merge;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use strum::Display;

pub static DEFAULT_PARSERS: LazyLock<IndexMap<Group, Parser>> =
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
                Parser::new(None, "<!-- 00 -->❌ Breaking".into(), false),
            ),
            (
                Group::Feature,
                Parser::new(
                    Some(feat_regex),
                    "<!-- 01 -->🚀 Features".into(),
                    false,
                ),
            ),
            (
                Group::Fix,
                Parser::new(
                    Some(fix_regex),
                    "<!-- 02 -->🐛 Bug Fixes".into(),
                    false,
                ),
            ),
            (
                Group::Revert,
                Parser::new(
                    Some(revert_regex),
                    "<!-- 03 -->◀️ Revert".into(),
                    false,
                ),
            ),
            (
                Group::Refactor,
                Parser::new(
                    Some(refactor_regex),
                    "<!-- 04 -->🚜 Refactor".into(),
                    false,
                ),
            ),
            (
                Group::Performance,
                Parser::new(
                    Some(perf_regex),
                    "<!-- 05 -->⚡ Performance".into(),
                    false,
                ),
            ),
            (
                Group::Documentation,
                Parser::new(
                    Some(doc_regex),
                    "<!-- 06 -->📚 Documentation".into(),
                    false,
                ),
            ),
            (
                Group::Style,
                Parser::new(
                    Some(style_regex),
                    "<!-- 07 -->🎨 Styling".into(),
                    false,
                ),
            ),
            (
                Group::Test,
                Parser::new(
                    Some(test_regex),
                    "<!-- 08 -->🧪 Testing".into(),
                    false,
                ),
            ),
            (
                Group::Chore,
                Parser::new(
                    Some(chore_regex),
                    "<!-- 09 -->🧹 Chore".into(),
                    false,
                ),
            ),
            (
                Group::CI,
                Parser::new(
                    Some(ci_regex),
                    "<!-- 10 -->⏩ CI/CD".into(),
                    false,
                ),
            ),
            (
                Group::Miscellaneous,
                Parser::new(
                    Some(misc_regex),
                    "<!-- 11 -->⚙️ Miscellaneous Tasks".into(),
                    false,
                ),
            ),
        ])
    });

/// Default changelog body template.
pub const DEFAULT_BODY: &str = r#"# [{{ version  }}]{% if tag_compare_link %}({{ tag_compare_link }}){% else %}({{ link }}){% endif %} - {{ timestamp | date(format="%Y-%m-%d") }}
{% for group, commits in commits | filter(attribute="merge_commit", value=false) | sort(attribute="group") | group_by(attribute="group") %}
### {{ group | striptags | trim }}
{% for commit in commits %}
{% if commit.breaking -%}
{% if commit.scope %}_({{ commit.scope }})_ {% endif -%}[**breaking**]: {{ commit.title }} [_({{ commit.short_id }})_]({{ commit.link }}){% if include_author %} ({{ commit.author_name }}){% endif %}
{% if commit.body -%}
> {{ commit.body }}
{% endif -%}
{% if commit.breaking_description -%}
> {{ commit.breaking_description }}
{% endif -%}
{% else -%}
- {% if commit.scope %}_({{ commit.scope }})_ {% endif %}{{ commit.title }} [_({{ commit.short_id }})_]({{ commit.link }}){% if include_author %} ({{ commit.author_name }}){% endif %}
{% endif -%}
{% endfor %}
{% endfor %}
 "#;

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

#[derive(Debug, Clone, Serialize, Deserialize, Merge, JsonSchema)]
pub struct Parser {
    #[schemars(with = "String")]
    #[serde(default, with = "serde_regex")]
    #[merge(strategy = merge::option::overwrite_none)]
    pub pattern: Option<Regex>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub title: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub skip: Option<bool>,
}

impl Parser {
    pub fn new(pattern: Option<Regex>, title: String, skip: bool) -> Self {
        Self {
            pattern,
            title: Some(title),
            skip: Some(skip),
        }
    }

    pub fn is_match(&self, msg: &str) -> bool {
        self.pattern
            .as_ref()
            .is_some_and(|p| p.is_match(msg.trim()))
    }

    /// Returns the parser's title and skip flag, applying defaults for
    /// any unset fields.
    pub fn title_and_skip(&self) -> (String, bool) {
        (
            self.title.as_deref().unwrap_or_default().into(),
            self.skip.unwrap_or_default(),
        )
    }
}

/// Rewords messages in changelog for targeted commit shas
#[derive(
    Debug, Clone, Default, JsonSchema, Serialize, Deserialize, Builder,
)]
#[builder(setter(into))]
pub struct RewordedCommit {
    /// Sha (or prefix) of the commit to reword. Matches any commit whose SHA
    /// starts with this value
    pub sha: String,
    /// The new message to display in changelog
    pub message: String,
}

/// Changelog configuration (applies to all packages)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Builder)]
#[builder(setter(into, strip_option), default)]
#[serde(default)] // Use default for missing fields
pub struct ChangelogConfig {
    /// Main changelog body template.
    pub body: String,
    /// Skips including merge commits in changelog
    pub skip_merge_commits: bool,
    /// Skips targeted commit shas (or prefixes) when generating next version
    /// and changelog. Each value matches any commit whose SHA starts with the
    /// provided value
    pub skip_shas: Option<Vec<String>>,
    /// Rewords commit messages for targeted shas when generated changelog.
    /// Each SHA can be a prefix - matches any commit whose SHA starts with the
    /// provided value
    pub reword: Option<Vec<RewordedCommit>>,
    /// Includes commit author name in default body template
    pub include_author: bool,
    /// Aggregates changelogs from prior prereleases when graduating
    pub aggregate_prereleases: bool,
    /// Default parsers for organizing commits into common groups e.g. feature,
    /// bug, etc. These can be turned off by setting the "skip" field to "true".
    /// Additionally you can modify the order by changing the tags in titles.
    /// For example to show bug fixes before features, change the fix group
    /// title to <!-- 01 -->🐛 Bug Fixes and the features title to
    /// <!-- 02 -->🚀 Features. Anything defined in this section will be merged
    /// with, and override, the pre-defined default parsers. So, for example,
    /// to only skip just CI commits, you only need to define the "ci" parser
    /// and set the "skip" field to true. All other parsers will remain as
    /// default.
    pub default_parsers: IndexMap<Group, Parser>,
    /// Additional parsers for grouping commits into non-default groups
    /// e.g. pattern="^special:" title="<!-- 00 -->Special" skip=false
    pub custom_parsers: Vec<Parser>,
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        Self {
            body: DEFAULT_BODY.into(),
            skip_merge_commits: true,
            skip_shas: None,
            reword: None,
            include_author: false,
            aggregate_prereleases: false,
            default_parsers: DEFAULT_PARSERS.clone(),
            custom_parsers: vec![],
        }
    }
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
