use semver::{BuildMetadata, Version};

use crate::{
    analyzer::version_strategy::{
        context::Context, date::get_date_parts, traits::VersionStrategy,
    },
    result::Result,
};

#[derive(Default)]
pub struct DateWithTimeVersionStrategy;

impl VersionStrategy for DateWithTimeVersionStrategy {
    fn calculate_next_version(&self, _ctx: &Context) -> Result<Version> {
        let [year, month, day, hr, min, sec, ..] = get_date_parts()?;
        let mut version = Version::new(year, month, day);
        version.build = BuildMetadata::new(&format!("{hr}.{min}.{sec}"))?;
        Ok(version)
    }
}
