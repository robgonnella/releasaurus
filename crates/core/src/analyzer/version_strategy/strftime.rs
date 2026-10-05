use crate::{
    analyzer::version_strategy::{context::Context, traits::VersionStrategy},
    forge::request::TagVersion,
    result::Result,
};

/// Ignores commits and the current tag; the version is the current UTC time.
pub struct StrftimeVersionStrategy {
    fmt: String,
}

impl StrftimeVersionStrategy {
    pub fn new(fmt: String) -> Self {
        Self { fmt }
    }
}

impl VersionStrategy for StrftimeVersionStrategy {
    fn calculate_next_version(&self, _ctx: &Context) -> Result<TagVersion> {
        TagVersion::strftime_now(&self.fmt)
    }
}
