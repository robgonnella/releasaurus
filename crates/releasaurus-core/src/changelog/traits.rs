//! Changelog related traits
use color_eyre::eyre::Result;

/// Defines the ability to generate a changelog string for package
pub trait Generator {
    fn generate(&self) -> Result<String>;
}

/// Defines the ability to return current version of package
pub trait CurrentVersion {
    fn current_version(&self) -> Option<String>;
}

/// Defines the ability to return next version based on analyzed commits
/// for a package
pub trait NextVersion {
    fn next_version(&self) -> Option<String>;
    fn next_is_breaking(&self) -> Result<bool>;
}
