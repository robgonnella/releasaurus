use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// Determines how prerelease identifiers should be appended to versions
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Default,
    Display,
    EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum PrereleaseStrategy {
    /// Adds numeric suffixes like `.1`, `.2`, etc. to prerelease identifiers.
    #[default]
    Versioned,
    /// Reuses the exact prerelease identifier without numeric suffixes
    Static,
}

/// Configurable prerelease settings for both default and package scopes
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default, deny_unknown_fields)]
pub struct PrereleaseConfig {
    /// Prerelease identifier (e.g., "alpha", "beta", "rc", "SNAPSHOT")
    pub suffix: String,
    /// How prerelease suffixes should be applied to versions
    pub strategy: PrereleaseStrategy,
}
