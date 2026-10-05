use crate::{
    analyzer::version_strategy::{context::Context, traits::VersionStrategy},
    forge::request::TagVersion,
    result::Result,
};

/// Strategy for semantic versions (stable or prerelease).
#[derive(Default)]
pub struct SemanticVersionStrategy;

impl VersionStrategy for SemanticVersionStrategy {
    fn calculate_next_version(&self, ctx: &Context) -> Result<TagVersion> {
        ctx.get_next_semantic_version()
    }
}
