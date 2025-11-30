use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Supported release types for updating package manifest files
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseType {
    #[default]
    Generic,
    Node,
    Rust,
    Python,
    Php,
    Ruby,
    Java,
}
