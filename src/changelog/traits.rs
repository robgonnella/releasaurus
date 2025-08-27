//! Changelog related traits
use color_eyre::eyre::Result;

#[derive(Debug)]
/// The output returned from Generator and Writer traits
pub struct Output {
    /// The entire changelog as a string
    pub changelog: String,
    /// The current version of latest release
    pub current_version: Option<String>,
    /// The next version as determined by conventional commits
    pub next_version: Option<String>,
}

/// Defines the ability to generate a changelog string for package
pub trait Generator {
    fn generate(&self) -> Result<Output>;
}

/// Writes the generated output to <package_path>/CHANGELOG.md
pub trait Writer {
    fn write(&self) -> Result<Output>;
}
