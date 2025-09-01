//! Changelog related traits

#[derive(Debug)]
pub struct ProjectedRelease {
    pub path: String,
    pub tag: String,
    pub sha: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    /// tag for this version
    pub tag: String,
    /// semver format sans any tag prefixes
    pub semver: semver::Version,
}

#[derive(Debug)]
/// The output returned from Generator and Writer traits
pub struct Output {
    /// The entire changelog as a string
    pub changelog: String,
    /// The current version of latest release
    pub current_version: Option<Version>,
    /// The next version as determined by conventional commits
    pub next_version: Option<Version>,
    /// The release that will be created
    pub projected_release: Option<ProjectedRelease>,
}
