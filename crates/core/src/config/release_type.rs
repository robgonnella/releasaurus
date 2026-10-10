use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

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
#[serde(rename_all = "snake_case")]
pub enum ReleaseType {
    #[default]
    Generic,
    Go,
    Helm,
    Node,
    Rust,
    Python,
    Php,
    Ruby,
    Java,
}

impl Display for ReleaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReleaseType::Generic => f.write_str("generic"),
            ReleaseType::Go => f.write_str("go"),
            ReleaseType::Helm => f.write_str("helm"),
            ReleaseType::Java => f.write_str("java"),
            ReleaseType::Node => f.write_str("node"),
            ReleaseType::Php => f.write_str("php"),
            ReleaseType::Python => f.write_str("python"),
            ReleaseType::Ruby => f.write_str("ruby"),
            ReleaseType::Rust => f.write_str("rust"),
        }
    }
}
