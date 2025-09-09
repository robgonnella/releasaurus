#[derive(Debug, Clone, PartialEq, Eq)]
/// Generic framework metadata for unsupported languages
pub struct GenericMetadata {
    /// Detected language/framework name
    pub framework_name: String,
    /// Main manifest file pattern
    pub manifest_pattern: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Generic package metadata
pub struct GenericPackageMetadata {}
