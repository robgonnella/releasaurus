use semver::Version;

use crate::{analyzer::version_strategy::context::Context, result::Result};

/// Trait for calculating the next version based on commits and current state.
pub trait VersionStrategy {
    /// Calculate the next version given the current context.
    fn calculate_next_version(&self, ctx: &Context) -> Result<Version>;
}
