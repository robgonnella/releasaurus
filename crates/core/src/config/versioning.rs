use chrono::{
    DateTime, TimeZone, Utc,
    format::{Item, Pad, Parsed, StrftimeItems},
};
use derive_builder::Builder;
use indexmap::IndexMap;
use merge::Merge;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::{str::FromStr, sync::LazyLock};
use strum::Display;

use crate::{config::prerelease::PrereleaseConfig, result::ReleasaurusError};

pub const DEFAULT_SKIP_MERGE_COMMITS: bool = true;
pub const DEFAULT_BREAKING_ALWAYS_INCREMENT_MAJOR: bool = true;
pub const DEFAULT_FEAT_ALWAYS_INCREMENT_MINOR: bool = true;
pub const DEFAULT_VERSION_TYPE: VersionType = VersionType::Semantic;

/// Determines what type of versioning to use (semantic, date, etc.)
#[derive(Debug, Default, Clone, JsonSchema, PartialEq, Eq)]
pub enum VersionType {
    #[default]
    #[serde(rename = "major.minor.patch")]
    Semantic,
    #[serde(rename = "major.minor.patch+timestamp.sha")]
    SemanticWithBuild,
    #[serde(rename = "year.month.day")]
    SemanticDate,
    #[serde(rename = "year.month.day+hour.minute.second")]
    SemanticDateWithTime,
    #[serde(rename = "year.month.day+hour.minute.second.micro")]
    SemanticDateWithTimeMicro,
    #[serde(untagged)]
    Strftime(#[schemars(pattern("%"))] String),
}

impl FromStr for VersionType {
    type Err = ReleasaurusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let version_type = match s {
            "major.minor.patch" => Self::Semantic,
            "major.minor.patch+timestamp.sha" => Self::SemanticWithBuild,
            "year.month.day" => Self::SemanticDate,
            "year.month.day+hour.minute.second" => Self::SemanticDateWithTime,
            "year.month.day+hour.minute.second.micro" => {
                Self::SemanticDateWithTimeMicro
            }
            fmt if fmt.contains('%') => {
                validate_strftime(fmt)?;
                Self::Strftime(fmt.to_string())
            }
            _ => {
                return Err(ReleasaurusError::invalid_config(format!(
                    "unknown version_type \"{s}\": expected one of \
                     major.minor.patch, major.minor.patch+timestamp.sha, \
                     year.month.day, year.month.day+hour.minute.second, \
                     year.month.day+hour.minute.second.micro, or a strftime \
                     format string such as \"%Y.%m.%d\""
                )));
            }
        };
        Ok(version_type)
    }
}

/// Fixed so validation errors are reproducible.
fn strftime_sample() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 5, 13, 7, 9).unwrap()
}

fn validate_strftime(fmt: &str) -> Result<(), ReleasaurusError> {
    if StrftimeItems::new(fmt).any(|item| matches!(item, Item::Error)) {
        return Err(ReleasaurusError::invalid_config(format!(
            "\"{fmt}\" is not a valid strftime format string"
        )));
    }

    // Space padding only appears for small values, which the sample
    // below may not hit, and git rejects spaces in tag names.
    if StrftimeItems::new(fmt)
        .any(|item| matches!(item, Item::Numeric(_, Pad::Space)))
    {
        return Err(ReleasaurusError::invalid_config(format!(
            "\"{fmt}\" pads a field with spaces (e.g. %e, %k, %_m), which \
             is not valid in a git tag name; use the zero-padded or \
             unpadded form instead (%d / %-d)"
        )));
    }

    let sample = strftime_sample().format(fmt).to_string();
    validate_git_tag_name(fmt, &sample)?;

    // existing tags are found by parsing their names
    parse_strftime(&sample, fmt).map_err(|e| {
        ReleasaurusError::invalid_config(format!(
            "\"{fmt}\" cannot be parsed back from the tag names it produces \
             (e.g. \"{sample}\"): {e}"
        ))
    })?;

    Ok(())
}

/// The `git check-ref-format` rules that hold whatever tag prefix is
/// prepended later.
fn validate_git_tag_name(
    fmt: &str,
    name: &str,
) -> Result<(), ReleasaurusError> {
    let invalid = |what: String| {
        ReleasaurusError::invalid_config(format!(
            "version_type \"{fmt}\" renders \"{name}\", which {what} and is \
             not a valid git tag name"
        ))
    };

    if let Some(c) = name.chars().find(|c| {
        c.is_ascii_control()
            || matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
    }) {
        return Err(invalid(format!("contains {c:?}")));
    }

    for seq in ["..", "@{", "//"] {
        if name.contains(seq) {
            return Err(invalid(format!("contains \"{seq}\"")));
        }
    }

    if name.ends_with('.') || name.ends_with('/') {
        return Err(invalid("ends with \".\" or \"/\"".into()));
    }

    if name
        .split('/')
        .any(|component| component.ends_with(".lock"))
    {
        return Err(invalid("has a component ending in \".lock\"".into()));
    }

    Ok(())
}

/// Fields the format omits take the first value of their period
/// (`%Y.%m` → the 1st at 00:00:00).
pub(crate) fn parse_strftime(
    raw: &str,
    fmt: &str,
) -> Result<DateTime<Utc>, ReleasaurusError> {
    let mut parsed = Parsed::new();
    chrono::format::parse(&mut parsed, raw, StrftimeItems::new(fmt))?;

    if parsed.to_naive_date().is_err() {
        if parsed.month().is_none() {
            parsed.set_month(1)?;
        }
        if parsed.day().is_none() {
            parsed.set_day(1)?;
        }
    }
    if parsed.to_naive_time().is_err() {
        if parsed.hour_div_12().is_none() {
            parsed.set_hour(0)?;
        }
        if parsed.minute().is_none() {
            parsed.set_minute(0)?;
        }
    }

    Ok(parsed.to_naive_datetime_with_offset(0)?.and_utc())
}

impl std::fmt::Display for VersionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Semantic => f.write_str("major.minor.patch"),
            Self::SemanticWithBuild => {
                f.write_str("major.minor.patch+timestamp.sha")
            }
            Self::SemanticDate => f.write_str("year.month.day"),
            Self::SemanticDateWithTime => {
                f.write_str("year.month.day+hour.minute.second")
            }
            Self::SemanticDateWithTimeMicro => {
                f.write_str("year.month.day+hour.minute.second.micro")
            }
            Self::Strftime(fmt) => f.write_str(fmt),
        }
    }
}

impl<'de> Deserialize<'de> for VersionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Serialize for VersionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl VersionType {
    /// Returns true for date-based version types, which derive the version
    /// from the current date/time rather than from commits. Semantic-only
    /// settings (prerelease, custom increment regexes) do not apply to
    /// these types.
    pub fn is_date_based(&self) -> bool {
        matches!(
            self,
            VersionType::SemanticDate
                | VersionType::SemanticDateWithTime
                | VersionType::SemanticDateWithTimeMicro
                | VersionType::Strftime(_)
        )
    }
}

/// Commit categories based on conventional commit types, used for grouping
/// changes in the changelog.
#[derive(
    Debug,
    Copy,
    Clone,
    Display,
    PartialEq,
    Eq,
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

    /// The value stored on [`Commit::group`][crate::orchestrator::Commit],
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

fn default_version_type() -> VersionType {
    DEFAULT_VERSION_TYPE
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
    /// Determines what kind of versioning to perform (semantic, date, etc).
    /// Default: major.minor.patch (semantic)
    #[merge(strategy = merge::option::overwrite_none)]
    #[schemars(default = "default_version_type")]
    pub version_type: Option<VersionType>,
    /// Prerelease configuration (suffix + strategy)
    // Excluded from the merge: `resolve_prerelease` has its own precedence
    // chain (config, then global CLI overrides, then per-package ones) and
    // assigns over whatever the surrounding merge produced.
    #[merge(skip)]
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
    #[schemars(with = "Option<String>")]
    #[serde(default, with = "serde_regex")]
    #[merge(strategy = merge::option::overwrite_none)]
    pub custom_major_increment_regex: Option<Regex>,
    /// Custom regex pattern matched against commit messages to trigger a
    /// minor version bump. This is additive — `feat:` commits always trigger
    /// minor bumps regardless of this setting. In TOML double-quoted strings,
    /// escape backslashes (e.g. `"\\[FEATURE\\]"` matches `[FEATURE]`).
    #[schemars(with = "Option<String>")]
    #[serde(default, with = "serde_regex")]
    #[merge(strategy = merge::option::overwrite_none)]
    pub custom_minor_increment_regex: Option<Regex>,
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

    #[test]
    fn version_type_from_str_round_trips_named_types() {
        for version_type in [
            VersionType::Semantic,
            VersionType::SemanticWithBuild,
            VersionType::SemanticDate,
            VersionType::SemanticDateWithTime,
            VersionType::SemanticDateWithTimeMicro,
        ] {
            assert_eq!(
                VersionType::from_str(&version_type.to_string()).unwrap(),
                version_type
            );
        }
    }

    #[test]
    fn version_type_from_str_rejects_unknown_names() {
        for s in ["", "semantic", "year.month.dya", "YYYY.MM.DD"] {
            let err = VersionType::from_str(s).unwrap_err();
            assert!(matches!(err, ReleasaurusError::InvalidConfig(_)), "{s:?}");
        }
    }

    #[test]
    fn version_type_from_str_accepts_strftime_formats() {
        for fmt in [
            "%Y.%m.%d",
            "%y.%-m.%-d",
            "%Y.%m",
            "%Y",
            "%Y.%j",
            "%Y.%m.%d.%H%M",
            "%Y%m%d-%H%M%S",
            "%Y/%m",
            "%Y.%B",
            "%Y%m%d%H%M%S%f",
            "%Y.%m.%d.%H%M%S%6f",
            "%Y.%m.%d.%H%M%S%.3f",
        ] {
            assert_eq!(
                VersionType::from_str(fmt).unwrap(),
                VersionType::Strftime(fmt.to_string()),
                "{fmt:?}"
            );
        }
    }

    #[test]
    fn version_type_from_str_rejects_irreversible_strftime_formats() {
        for fmt in ["%Q", "%Y.%A", "%m.%d", "%H.%M", "%Y.%I", "%Y.%W", "%s"] {
            let err = VersionType::from_str(fmt).unwrap_err();
            assert!(
                matches!(err, ReleasaurusError::InvalidConfig(_)),
                "{fmt:?}"
            );
        }
    }

    #[test]
    fn version_type_from_str_rejects_formats_that_are_not_git_tag_names() {
        for fmt in [
            "%Y %m",
            "%T",
            "%c",
            "%Y:%m",
            "%Y..%m",
            "%Y[%m]",
            "%Y.%m.",
            "%Y.%m.lock",
            "%Y/%m.lock/%d",
            "%Y@{%m",
            "%Y*",
            "%Y?",
            "%Y~%m",
            "%Y^%m",
            "%Y\\%m",
            "%Y//%m",
            "%Y/",
            "%Y\t%m",
            // space-padded: reject formats that render with spaces
            "%Y.%m.%d.%k",
            "%Y.%m.%d.%_H",
            "%Y.%_m.%d",
            "%Y.%_j",
        ] {
            let err = VersionType::from_str(fmt).unwrap_err();
            assert!(
                matches!(err, ReleasaurusError::InvalidConfig(_)),
                "{fmt:?}"
            );
        }
    }

    #[test]
    fn version_type_serde_round_trips_and_rejects_typos() {
        #[derive(Serialize, Deserialize)]
        struct Wrapper {
            version_type: VersionType,
        }

        let w: Wrapper =
            toml::from_str(r#"version_type = "%Y.%m.%d""#).unwrap();
        assert_eq!(w.version_type, VersionType::Strftime("%Y.%m.%d".into()));
        assert_eq!(
            toml::to_string(&w).unwrap().trim(),
            r#"version_type = "%Y.%m.%d""#
        );

        let w: Wrapper =
            toml::from_str(r#"version_type = "year.month.day""#).unwrap();
        assert_eq!(w.version_type, VersionType::SemanticDate);

        assert!(
            toml::from_str::<Wrapper>(r#"version_type = "year.month.dya""#)
                .is_err()
        );
    }

    #[test]
    fn parse_strftime_defaults_omitted_fields_to_start_of_period() {
        let cases = [
            ("2026.10.05", "%Y.%m.%d", (2026, 10, 5, 0, 0, 0)),
            ("2026.10", "%Y.%m", (2026, 10, 1, 0, 0, 0)),
            ("2026", "%Y", (2026, 1, 1, 0, 0, 0)),
            ("26.10.5.1307", "%y.%-m.%-d.%H%M", (2026, 10, 5, 13, 7, 0)),
            ("2026.278", "%Y.%j", (2026, 10, 5, 0, 0, 0)),
        ];
        for (raw, fmt, (y, mo, d, h, mi, s)) in cases {
            assert_eq!(
                parse_strftime(raw, fmt).unwrap(),
                Utc.with_ymd_and_hms(y, mo, d, h, mi, s).unwrap(),
                "{raw:?} / {fmt:?}"
            );
        }
    }

    /// Tags made under `year.month.day` are unpadded.
    #[test]
    fn parse_strftime_accepts_unpadded_fields() {
        assert_eq!(
            parse_strftime("2026.10.5", "%Y.%m.%d").unwrap(),
            Utc.with_ymd_and_hms(2026, 10, 5, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn parse_strftime_rejects_names_not_matching_format() {
        for raw in ["2026-10-05", "2026.10", "2026.10.05-rc.1", "abc"] {
            assert!(parse_strftime(raw, "%Y.%m.%d").is_err(), "{raw:?}");
        }
    }
}
