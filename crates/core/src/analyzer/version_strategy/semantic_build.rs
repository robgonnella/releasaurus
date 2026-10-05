use semver::BuildMetadata;

use crate::{
    analyzer::version_strategy::{context::Context, traits::VersionStrategy},
    forge::request::TagVersion,
    result::Result,
};

/// Strategy for when all versions should contain build metadata
/// Build metadata consists of {timestamp}.{short_sha}
#[derive(Default)]
pub struct SemanticBuildVersionStrategy;

impl VersionStrategy for SemanticBuildVersionStrategy {
    fn calculate_next_version(&self, ctx: &Context) -> Result<TagVersion> {
        let mut version = ctx.get_next_semantic_version()?;
        let build_metadata = format!("{}.{}", ctx.timestamp, ctx.short_sha);
        version.set_build(BuildMetadata::new(&build_metadata)?);
        Ok(version)
    }
}
