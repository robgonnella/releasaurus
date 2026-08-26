use derive_builder::Builder;
use indexmap::IndexMap;
use merge::Merge;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::prerelease::PrereleaseConfig;

mod parser;
mod version_type;

pub use parser::{Group, MAX_PARSER_ORDER, NAMED_PARSERS, Parser, ParserList};
pub use version_type::{DEFAULT_VERSION_TYPE, VersionType, parse_strftime};

pub const DEFAULT_SKIP_MERGE_COMMITS: bool = true;
pub const DEFAULT_BREAKING_ALWAYS_INCREMENT_MAJOR: bool = true;
pub const DEFAULT_FEAT_ALWAYS_INCREMENT_MINOR: bool = true;

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
