use crate::analyzer::release::Release;

/// Package after commit analysis.
///
/// `release` is `None` when no commits triggered a version bump.
#[derive(Debug)]
pub struct AnalyzedPackage {
    pub name: String,
    pub release: Option<Release>,
}
