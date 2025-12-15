use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Determines how prerelease identifiers should be appended to versions.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq,
)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum PrereleaseStrategy {
    /// Adds numeric suffixes like `.1`, `.2`, etc. to prerelease identifiers.
    #[default]
    Versioned,
    /// Reuses the exact prerelease identifier without numeric suffixes.
    Static,
}


/// User-configurable prerelease settings at global and package scopes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default)]
pub struct PrereleaseConfig {
    /// Prerelease identifier (e.g., "alpha", "beta", "rc", "SNAPSHOT").
    pub suffix: Option<String>,
    /// How prerelease suffixes should be applied to versions.
    pub strategy: PrereleaseStrategy,
}

impl PrereleaseConfig {
    fn sanitized_suffix(&self) -> Option<String> {
        self.suffix
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    /// Resolves this config against an optional override, returning the final
    /// prerelease settings when a suffix is available.
    pub fn resolve_with_override(
        &self,
        override_cfg: Option<&PrereleaseConfig>,
    ) -> Option<PrereleaseConfig> {
        let candidate = override_cfg.unwrap_or(self);
        candidate.sanitized_suffix().map(|suffix| {
            let mut resolved = candidate.clone();
            resolved.suffix = Some(suffix);
            resolved
        })
    }

    /// Returns the sanitized suffix for configs that have been resolved.
    pub fn resolved_suffix(&self) -> &str {
        self.suffix
            .as_deref()
            .expect("resolved prerelease config must include suffix")
    }
}
