use semver::{BuildMetadata, Version};

use crate::{
    analyzer::version_strategy::{
        context::Context, date::DateParts, traits::VersionStrategy,
    },
    result::Result,
};

#[derive(Default)]
pub struct DateWithTimeVersionStrategy;

impl VersionStrategy for DateWithTimeVersionStrategy {
    fn calculate_next_version(&self, _ctx: &Context) -> Result<Version> {
        let parts = DateParts::now();
        let mut version = Version::new(parts.year, parts.month, parts.day);
        version.build = BuildMetadata::new(&parts.time_build_metadata())?;
        Ok(version)
    }
}
