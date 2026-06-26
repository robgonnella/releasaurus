use derive_builder::Builder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::{
    changelog::ChangelogConfig, package::PackageConfig,
    prerelease::PrereleaseConfig,
};

/// Global configuration that applies to all packages
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Builder)]
#[builder(setter(into, strip_option), default)]
#[serde(default)] // Use default for missing fields
pub struct GlobalConfig {
    /// Global prerelease configuration (suffix + strategy). Packages can
    /// override this configuration
    pub prerelease: PrereleaseConfig,
    /// Global config to auto start next release for all packages. Packages
    /// can override this configuration
    pub auto_start_next: Option<bool>,
    /// Global config to always increments major version on breaking commits
    /// Packages can override this configuration
    pub breaking_always_increment_major: bool,
    /// Global config to always increments minor version on feature commits
    /// Packages can override this configuration
    pub features_always_increment_minor: bool,
    /// Custom regex pattern matched against commit messages to trigger a
    /// major version bump. This is additive — breaking change commits always
    /// trigger major bumps regardless of this setting. In TOML double-quoted
    /// strings, escape backslashes (e.g. `"\\[BREAKING\\]"` matches
    /// `[BREAKING]`).
    pub custom_major_increment_regex: Option<String>,
    /// Custom regex pattern matched against commit messages to trigger a
    /// minor version bump. This is additive — `feat:` commits always trigger
    /// minor bumps regardless of this setting. In TOML double-quoted strings,
    /// escape backslashes (e.g. `"\\[FEATURE\\]"` matches `[FEATURE]`).
    pub custom_minor_increment_regex: Option<String>,
    /// Global changelog generation settings applied to all packages.
    /// Packages can override this configuration
    pub changelog: Option<ChangelogConfig>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            prerelease: PrereleaseConfig::default(),
            auto_start_next: None,
            breaking_always_increment_major: true,
            features_always_increment_minor: true,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            changelog: None,
        }
    }
}

impl GlobalConfig {
    pub fn auto_start_next(&self, package: &PackageConfig) -> bool {
        package
            .auto_start_next
            .or(self.auto_start_next)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_start_next_uses_package_override() {
        let config = GlobalConfig {
            auto_start_next: Some(false),
            ..Default::default()
        };
        let package = PackageConfig {
            auto_start_next: Some(true),
            ..Default::default()
        };

        assert!(config.auto_start_next(&package));
    }

    #[test]
    fn auto_start_next_uses_global_when_package_not_set() {
        let config = GlobalConfig {
            auto_start_next: Some(true),
            ..Default::default()
        };
        let package = PackageConfig::default();

        assert!(config.auto_start_next(&package));
    }

    #[test]
    fn auto_start_next_defaults_to_false() {
        let config = GlobalConfig::default();
        let package = PackageConfig::default();

        assert!(!config.auto_start_next(&package));
    }

    #[test]
    fn increment_flags_default_to_true() {
        let config = GlobalConfig::default();

        assert!(config.breaking_always_increment_major);
        assert!(config.features_always_increment_minor);
    }
}
