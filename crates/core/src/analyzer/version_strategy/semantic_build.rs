use semver::{BuildMetadata, Version};

use crate::{
    analyzer::version_strategy::{context::Context, traits::VersionStrategy},
    result::Result,
};

/// Strategy for when all versions should contain build metadata
/// Build metadata consists of {timestamp}.{short_sha}
#[derive(Default)]
pub struct SemanticBuildVersionStrategy;

impl VersionStrategy for SemanticBuildVersionStrategy {
    fn calculate_next_version(&self, ctx: &Context) -> Result<Version> {
        let mut version = ctx.get_next_semantic_version()?;
        let build_metadata = format!("{}.{}", ctx.timestamp, ctx.short_sha);
        version.build = BuildMetadata::new(&build_metadata)?;
        Ok(version)
    }
}
