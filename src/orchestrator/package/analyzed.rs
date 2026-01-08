use crate::analyzer::release::Release;

/// Represents a fully analyzed package with potential next release
#[derive(Debug)]
pub struct AnalyzedPackage {
    pub name: String,
    pub release: Option<Release>,
}
