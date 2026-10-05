use semver::{BuildMetadata, Version};

use crate::{
    analyzer::version_strategy::{
        context::Context, semantic_date::DateParts, traits::VersionStrategy,
    },
    forge::request::TagVersion,
    result::Result,
};

#[derive(Default)]
pub struct DateWithTimeMicroVersionStrategy;

impl VersionStrategy for DateWithTimeMicroVersionStrategy {
    fn calculate_next_version(&self, _ctx: &Context) -> Result<TagVersion> {
        let parts = DateParts::now();
        let mut version = Version::new(parts.year, parts.month, parts.day);
        // Padded to the six digits the microsecond field can hold, for the
        // same lexical-sort reason as the time segments.
        version.build = BuildMetadata::new(&format!(
            "{}.{:06}",
            parts.time_build_metadata(),
            parts.micro
        ))?;
        Ok(TagVersion::Semantic(version))
    }
}
