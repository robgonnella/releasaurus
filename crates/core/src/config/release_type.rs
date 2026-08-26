use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Config specific to the helm updater
#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    Hash,
)]
#[serde(default, deny_unknown_fields)]
pub struct HelmConfig {
    /// Also write the release version to the chart's top-level
    /// `appVersion`. Only a value containing an `X.Y.Z` is rewritten.
    pub update_app_version: bool,
    /// Write the full tag name, `tag_prefix` included, to `appVersion`
    /// instead of the bare version. Has no effect unless
    /// `update_app_version` is set.
    pub app_version_tag_prefix: bool,
}

/// Supported release types for updating package manifest files
#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    Hash,
)]
#[serde(rename_all = "snake_case", from = "ReleaseTypeRepr")]
pub enum ReleaseType {
    #[default]
    Generic,
    Go,
    Helm(HelmConfig),
    Node,
    Rust,
    Python,
    Php,
    Ruby,
    Java,
}

// The accepted input forms, which `ReleaseType` alone cannot express: a
// variant holding config deserializes from a table only, but `"helm"` must
// still parse. Also the source of `ReleaseType`'s JSON schema.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum ReleaseTypeRepr {
    Generic,
    Go,
    Helm,
    Node,
    Rust,
    Python,
    Php,
    Ruby,
    Java,
    #[serde(untagged)]
    HelmTable {
        helm: HelmConfig,
    },
}

impl From<ReleaseTypeRepr> for ReleaseType {
    fn from(repr: ReleaseTypeRepr) -> Self {
        match repr {
            ReleaseTypeRepr::Generic => Self::Generic,
            ReleaseTypeRepr::Go => Self::Go,
            ReleaseTypeRepr::Helm => Self::Helm(HelmConfig::default()),
            ReleaseTypeRepr::HelmTable { helm } => Self::Helm(helm),
            ReleaseTypeRepr::Java => Self::Java,
            ReleaseTypeRepr::Node => Self::Node,
            ReleaseTypeRepr::Php => Self::Php,
            ReleaseTypeRepr::Python => Self::Python,
            ReleaseTypeRepr::Ruby => Self::Ruby,
            ReleaseTypeRepr::Rust => Self::Rust,
        }
    }
}

impl Display for ReleaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReleaseType::Generic => f.write_str("generic"),
            ReleaseType::Go => f.write_str("go"),
            ReleaseType::Helm(_) => f.write_str("helm"),
            ReleaseType::Java => f.write_str("java"),
            ReleaseType::Node => f.write_str("node"),
            ReleaseType::Php => f.write_str("php"),
            ReleaseType::Python => f.write_str("python"),
            ReleaseType::Ruby => f.write_str("ruby"),
            ReleaseType::Rust => f.write_str("rust"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct TestConfig {
        release_type: ReleaseType,
    }

    fn parse(value: &str) -> Result<ReleaseType, toml::de::Error> {
        toml::from_str::<TestConfig>(&format!("release_type = {value}"))
            .map(|c| c.release_type)
    }

    #[test]
    fn parses_bare_helm_name_with_default_config() {
        assert_eq!(
            parse(r#""helm""#).unwrap(),
            ReleaseType::Helm(HelmConfig::default())
        );
    }

    #[test]
    fn parses_helm_table_with_update_app_version() {
        assert_eq!(
            parse("{ helm = { update_app_version = true } }").unwrap(),
            ReleaseType::Helm(HelmConfig {
                update_app_version: true,
                ..Default::default()
            })
        );
    }

    #[test]
    fn parses_helm_table_with_app_version_tag_prefix() {
        assert_eq!(
            parse(
                "{ helm = { update_app_version = true, \
                 app_version_tag_prefix = true } }"
            )
            .unwrap(),
            ReleaseType::Helm(HelmConfig {
                update_app_version: true,
                app_version_tag_prefix: true,
            })
        );
    }

    #[test]
    fn parses_empty_helm_table_as_default_config() {
        assert_eq!(
            parse("{ helm = {} }").unwrap(),
            ReleaseType::Helm(HelmConfig::default())
        );
    }

    #[test]
    fn rejects_unknown_key_in_helm_table() {
        assert!(parse("{ helm = { update_app_versions = true } }").is_err());
    }

    #[test]
    fn rejects_table_for_another_release_type() {
        assert!(parse("{ node = {} }").is_err());
    }

    #[test]
    fn rejects_unknown_name() {
        assert!(parse(r#""helmet""#).is_err());
    }

    #[test]
    fn parses_every_name() {
        let cases = [
            ("generic", ReleaseType::Generic),
            ("go", ReleaseType::Go),
            ("helm", ReleaseType::Helm(HelmConfig::default())),
            ("java", ReleaseType::Java),
            ("node", ReleaseType::Node),
            ("php", ReleaseType::Php),
            ("python", ReleaseType::Python),
            ("ruby", ReleaseType::Ruby),
            ("rust", ReleaseType::Rust),
        ];

        for (name, expected) in cases {
            assert_eq!(parse(&format!("\"{name}\"")).unwrap(), expected);
            assert_eq!(expected.to_string(), name);
        }
    }

    #[test]
    fn displays_helm_regardless_of_config() {
        let release_type = ReleaseType::Helm(HelmConfig {
            update_app_version: true,
            app_version_tag_prefix: true,
        });

        assert_eq!(release_type.to_string(), "helm");
    }
}
