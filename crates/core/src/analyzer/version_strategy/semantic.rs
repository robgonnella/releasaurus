use semver::Version;

use crate::{
    analyzer::version_strategy::{context::Context, traits::VersionStrategy},
    result::Result,
};

/// Strategy for semantic versions (stable or prerelease).
#[derive(Default)]
pub struct SemanticVersionStrategy;

impl VersionStrategy for SemanticVersionStrategy {
    fn calculate_next_version(&self, ctx: &Context) -> Result<Version> {
        ctx.get_next_semantic_version()
    }
}
