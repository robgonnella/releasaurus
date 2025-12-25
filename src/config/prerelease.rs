use clap::ValueEnum;
use color_eyre::eyre::eyre;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Result;

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
    ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum PrereleaseStrategy {
    /// Adds numeric suffixes like `.1`, `.2`, etc. to prerelease identifiers.
    #[default]
    Versioned,
    /// Reuses the exact prerelease identifier without numeric suffixes
    Static,
}

/// Configurable prerelease settings for both global and package scopes
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default)]
pub struct PrereleaseConfig {
    /// Prerelease identifier (e.g., "alpha", "beta", "rc", "SNAPSHOT")
    pub suffix: Option<String>,
    /// How prerelease suffixes should be applied to versions
    pub strategy: PrereleaseStrategy,
}

impl PrereleaseConfig {
    /// Returns the suffix for configs that have been resolved
    pub fn suffix(&self) -> Result<&str> {
        self.suffix
            .as_deref()
            .ok_or(eyre!("resolved prerelease config must include suffix"))
    }
}
