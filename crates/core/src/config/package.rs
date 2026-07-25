use std::sync::LazyLock;

use derive_builder::Builder;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    config::{
        changelog::ChangelogConfig, release_type::ReleaseType,
        versioning::VersioningConfig,
    },
    result::{ReleasaurusError, Result},
};

/// Default generic version matcher regex pattern
pub const GENERIC_VERSION_REGEX_PATTERN: &str = r#"(?mi)(?<start>.*version"?:?\s*=?\s*['"]?)(?<version>\d+\.\d+\.\d+-?.*?)(?<end>['",].*)?$"#;

/// Compiled version of GENERIC_VERSION_REGEX_PATTERN
pub static GENERIC_VERSION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(GENERIC_VERSION_REGEX_PATTERN).unwrap());

/// Default tag prefix for package
pub const DEFAULT_TAG_PREFIX: &str = "v";

/// Additional manifest specification that accepts either a string path or full
/// config. Allows users to specify version files in a concise way while still
/// supporting custom regex patterns when needed.
///
/// # Examples
///
/// Simple string path (uses default GENERIC_VERSION_REGEX):
/// ```toml
/// additional_manifest_files = ["VERSION", "README.md"]
/// ```
///
/// Full config with custom regex:
/// ```toml
/// additional_manifest_files = [
///     { path = "VERSION.txt", version_regex = "version:\\s*(\\d+\\.\\d+\\.\\d+)" }
/// ]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum AdditionalManifestSpec {
    /// Simple string path - uses default GENERIC_VERSION_REGEX
    Path(String),
    /// Full configuration with optional custom regex
    Full(AdditionalManifest),
}

impl AdditionalManifestSpec {
    /// Converts the spec into an AdditionalManifest.
    /// Path variants are converted to use the default GENERIC_VERSION_REGEX
    /// pattern. Full variants with None for version_regex also get the default
    /// pattern. After conversion, version_regex is always Some.
    pub fn into_manifest(self) -> AdditionalManifest {
        match self {
            AdditionalManifestSpec::Path(path) => AdditionalManifest {
                path,
                version_regex: Some(GENERIC_VERSION_REGEX_PATTERN.to_string()),
            },
            AdditionalManifestSpec::Full(mut manifest) => {
                // Normalize None to default pattern
                if manifest.version_regex.is_none() {
                    manifest.version_regex =
                        Some(GENERIC_VERSION_REGEX_PATTERN.to_string());
                }
                manifest
            }
        }
    }
}

/// Additional manifest configuration for version updates on arbitrary files.
/// This is the internal representation after conversion from AdditionalManifestSpec.
#[derive(
    Debug, Default, Clone, Serialize, Deserialize, JsonSchema, Builder,
)]
#[serde(deny_unknown_fields)]
pub struct AdditionalManifest {
    /// The path to the manifest file relative to package path
    pub path: String,
    /// The regex to use to match and replace versions.
    ///
    /// Must define a `version` capture group — config resolution
    /// rejects patterns without one. Only that group's text is
    /// substituted; the rest of the match is left intact. Defaults to
    /// [`GENERIC_VERSION_REGEX_PATTERN`] when omitted.
    pub version_regex: Option<String>,
}

/// Sub-package definition allowing grouping of packages under a parent package
/// configuration. Sub-packages share changelog, tag, and release with the
/// parent package definition but receive independent manifest version file
/// updates according to their defined release type
#[derive(
    Debug, Default, Clone, Serialize, Deserialize, JsonSchema, Builder,
)]
#[serde(deny_unknown_fields)]
pub struct SubPackage {
    /// Name for this sub-package (default derived from path if not provided).
    /// For proper manifest version file updates this should match the
    /// canonical name field in the release_type manifest file.
    /// i.e. name = "..." in Cargo.toml or "name": "..." in package.json
    pub name: String,
    /// Path to the subpackage directory relative to the workspace_root of
    /// the parent package
    pub path: String,
    /// [`ReleaseType`] type for determining which version files to update
    pub release_type: Option<ReleaseType>,
}

/// Package configuration for multi-package repositories and monorepos
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Builder)]
#[serde(default, deny_unknown_fields)] // Use default for missing fields
#[builder(setter(into, strip_option), default)]
pub struct PackageConfig {
    /// Name for this package (default derived from path if not provided). For
    /// proper manifest version file updates this should match the
    /// canonical name field in the release_type manifest file.
    /// i.e. name = "..." in Cargo.toml or "name": "..." in package.json
    pub name: String,
    /// Path to the workspace root directory for this package relative to the
    /// repository root
    pub workspace_root: String,
    /// Path to package directory relative to workspace_root
    pub path: String,
    /// [`ReleaseType`] type for determining which version files to update
    pub release_type: Option<ReleaseType>,
    /// Git tag prefix for this package (e.g., "v" or "api-v")
    pub tag_prefix: Option<String>,
    /// Groups sub-packages under a single release. Each will share changelog,
    /// tag, and release, but will receive independent manifest version updates
    /// according to their type
    pub sub_packages: Option<Vec<SubPackage>>,
    /// Additional directory paths to include commits from
    pub additional_paths: Option<Vec<String>>,
    /// Additional paths to generic version manifest files to update. Paths must
    /// be relative to the package path. Accepts either simple string paths or
    /// full config objects with custom regex patterns.
    pub additional_manifest_files: Option<Vec<AdditionalManifestSpec>>,
    /// Changelog generation settings applied to target package.
    pub changelog: Option<ChangelogConfig>,
    /// Versioning config for calculating next versions of packages
    pub versioning: Option<VersioningConfig>,
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            name: "".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            sub_packages: None,
            release_type: None,
            tag_prefix: None,
            additional_paths: None,
            additional_manifest_files: None,
            changelog: None,
            versioning: None,
        }
    }
}

impl PackageConfig {
    pub fn tag_prefix(&self) -> Result<String> {
        self.tag_prefix.clone().ok_or_else(|| {
            ReleasaurusError::invalid_config(format!(
                "failed to resolve tag prefix for package: {}",
                self.name
            ))
        })
    }
}

impl From<SubPackage> for PackageConfig {
    fn from(value: SubPackage) -> Self {
        Self {
            path: value.path,
            release_type: value.release_type,
            ..Default::default()
        }
    }
}

impl From<&SubPackage> for PackageConfig {
    fn from(value: &SubPackage) -> Self {
        Self {
            path: value.path.clone(),
            release_type: value.release_type,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_string_path_format() {
        let toml = r#"
            additional_manifest_files = ["VERSION", "README.md"]
        "#;

        #[derive(Deserialize)]
        struct TestConfig {
            additional_manifest_files: Option<Vec<AdditionalManifestSpec>>,
        }

        let config: TestConfig = toml::from_str(toml).unwrap();
        let specs = config.additional_manifest_files.unwrap();

        assert_eq!(specs.len(), 2);

        let manifest1 = specs[0].clone().into_manifest();
        assert_eq!(manifest1.path, "VERSION");
        assert_eq!(
            manifest1.version_regex,
            Some(GENERIC_VERSION_REGEX_PATTERN.to_string())
        );

        let manifest2 = specs[1].clone().into_manifest();
        assert_eq!(manifest2.path, "README.md");
        assert_eq!(
            manifest2.version_regex,
            Some(GENERIC_VERSION_REGEX_PATTERN.to_string())
        );
    }

    #[test]
    fn deserializes_full_object_format() {
        let toml = r#"
            [[additional_manifest_files]]
            path = "VERSION"
            version_regex = "version:\\s*(\\d+\\.\\d+\\.\\d+)"
        "#;

        #[derive(Deserialize)]
        struct TestConfig {
            additional_manifest_files: Option<Vec<AdditionalManifestSpec>>,
        }

        let config: TestConfig = toml::from_str(toml).unwrap();
        let specs = config.additional_manifest_files.unwrap();

        assert_eq!(specs.len(), 1);

        let manifest = specs[0].clone().into_manifest();
        assert_eq!(manifest.path, "VERSION");
        assert_eq!(
            manifest.version_regex,
            Some("version:\\s*(\\d+\\.\\d+\\.\\d+)".to_string())
        );
    }

    #[test]
    fn deserializes_mixed_format() {
        let toml = r#"
            additional_manifest_files = [
                "VERSION",
                { path = "config.yml", version_regex = "v:\\s*(\\d+\\.\\d+\\.\\d+)" }
            ]
        "#;

        #[derive(Deserialize)]
        struct TestConfig {
            additional_manifest_files: Option<Vec<AdditionalManifestSpec>>,
        }

        let config: TestConfig = toml::from_str(toml).unwrap();
        let specs = config.additional_manifest_files.unwrap();

        assert_eq!(specs.len(), 2);

        let manifest1 = specs[0].clone().into_manifest();
        assert_eq!(manifest1.path, "VERSION");
        assert_eq!(
            manifest1.version_regex,
            Some(GENERIC_VERSION_REGEX_PATTERN.to_string())
        );

        let manifest2 = specs[1].clone().into_manifest();
        assert_eq!(manifest2.path, "config.yml");
        assert_eq!(
            manifest2.version_regex,
            Some("v:\\s*(\\d+\\.\\d+\\.\\d+)".to_string())
        );
    }

    #[test]
    fn deserializes_full_package_config_with_manifest_files() {
        let toml = r#"
            [[package]]
            path = "."
            release_type = "rust"
            additional_manifest_files = ["VERSION", "README.md"]

            [[package]]
            path = "packages/api"
            release_type = "node"
            additional_manifest_files = [
                "VERSION",
                { path = "config.yml", version_regex = "v:\\s*(\\d+\\.\\d+\\.\\d+)" }
            ]
        "#;

        #[derive(Deserialize)]
        struct TestConfig {
            package: Vec<PackageConfig>,
        }

        let config: TestConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.package.len(), 2);

        // First package - simple string format
        let pkg1_specs = config.package[0]
            .additional_manifest_files
            .as_ref()
            .unwrap();
        assert_eq!(pkg1_specs.len(), 2);
        let manifest1 = pkg1_specs[0].clone().into_manifest();
        assert_eq!(manifest1.path, "VERSION");
        assert_eq!(
            manifest1.version_regex,
            Some(GENERIC_VERSION_REGEX_PATTERN.to_string())
        );

        // Second package - mixed format
        let pkg2_specs = config.package[1]
            .additional_manifest_files
            .as_ref()
            .unwrap();
        assert_eq!(pkg2_specs.len(), 2);
        let manifest2_1 = pkg2_specs[0].clone().into_manifest();
        assert_eq!(manifest2_1.path, "VERSION");
        assert_eq!(
            manifest2_1.version_regex,
            Some(GENERIC_VERSION_REGEX_PATTERN.to_string())
        );

        let manifest2_2 = pkg2_specs[1].clone().into_manifest();
        assert_eq!(manifest2_2.path, "config.yml");
        assert_eq!(
            manifest2_2.version_regex,
            Some("v:\\s*(\\d+\\.\\d+\\.\\d+)".to_string())
        );
    }

    #[test]
    fn normalizes_full_variant_with_none_to_default_pattern() {
        // Test that Full variant with None gets normalized to default pattern
        let spec = AdditionalManifestSpec::Full(AdditionalManifest {
            path: "VERSION".to_string(),
            version_regex: None,
        });

        let manifest = spec.into_manifest();
        assert_eq!(manifest.path, "VERSION");
        assert_eq!(
            manifest.version_regex,
            Some(GENERIC_VERSION_REGEX_PATTERN.to_string())
        );
    }

    #[test]
    fn preserves_full_variant_custom_regex() {
        // Test that Full variant with custom regex is preserved
        let custom_pattern = "custom:\\s*(\\d+\\.\\d+\\.\\d+)".to_string();
        let spec = AdditionalManifestSpec::Full(AdditionalManifest {
            path: "config.yml".to_string(),
            version_regex: Some(custom_pattern.clone()),
        });

        let manifest = spec.into_manifest();
        assert_eq!(manifest.path, "config.yml");
        assert_eq!(manifest.version_regex, Some(custom_pattern));
    }
}
