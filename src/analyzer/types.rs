//! Changelog related traits

#[derive(Debug)]
pub struct ProjectedRelease {
    pub path: String,
    pub tag: String,
    pub sha: String,
    pub notes: String,
}

#[derive(Debug)]
/// The output returned from Generator and Writer traits
pub struct Output {
    /// The entire changelog as a string
    pub changelog: String,
    /// The current version of latest release
    pub current_version: Option<String>,
    /// The next version as determined by conventional commits
    pub next_version: Option<String>,
    /// The release that will be created
    pub projected_release: Option<ProjectedRelease>,
}
