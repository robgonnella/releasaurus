use semver::Version;

use crate::{
    analyzer::{
        helpers,
        version_strategy::{context::Context, traits::VersionStrategy},
    },
    config::prerelease::PrereleaseStrategy,
    result::Result,
};

/// Strategy for static prerelease versions (e.g., SNAPSHOT, dev).
pub struct StaticPrereleaseStrategy {
    identifier: String,
}

impl StaticPrereleaseStrategy {
    /// Create a new static prerelease strategy with the given identifier.
    pub fn new(identifier: String) -> Self {
        Self { identifier }
    }
}

impl VersionStrategy for StaticPrereleaseStrategy {
    fn calculate_next_version(&self, ctx: &Context) -> Result<Version> {
        if let Some(current) = ctx.current_tag {
            if current.semver.pre.is_empty() {
                // Starting new prerelease from stable
                log::info!(
                    "static prerelease strategy: starting new prerelease from stable {}",
                    current.semver
                );
                let version_updater = ctx.create_version_updater()?;
                let next_stable =
                    version_updater.increment(&current.semver, ctx.commits);
                helpers::add_prerelease(
                    next_stable,
                    &self.identifier,
                    PrereleaseStrategy::Static,
                )
            } else {
                // Currently in a prerelease
                let current_pre_id =
                    current.semver.pre.as_str().split('.').next().unwrap_or("");

                if current_pre_id == self.identifier {
                    // Same static identifier - increment version and re-add suffix
                    log::info!(
                        "static prerelease strategy: incrementing prerelease {}",
                        current.semver
                    );
                    let mut version =
                        helpers::graduate_prerelease(&current.semver);
                    let version_updater = ctx.create_version_updater()?;
                    version = version_updater.increment(&version, ctx.commits);
                    helpers::add_prerelease(
                        version,
                        &self.identifier,
                        PrereleaseStrategy::Static,
                    )
                } else {
                    // Different prerelease identifier - switch to new one
                    log::info!(
                        "static prerelease strategy: switching from {} to {}",
                        current_pre_id,
                        self.identifier
                    );
                    let stable_current =
                        helpers::graduate_prerelease(&current.semver);
                    let version_updater = ctx.create_version_updater()?;
                    let stable_next =
                        version_updater.increment(&stable_current, ctx.commits);
                    helpers::add_prerelease(
                        stable_next,
                        &self.identifier,
                        PrereleaseStrategy::Static,
                    )
                }
            }
        } else {
            // First release as prerelease
            log::info!(
                "static prerelease strategy: first release as prerelease"
            );
            let version = Version::parse("0.1.0")?;
            helpers::add_prerelease(
                version,
                &self.identifier,
                PrereleaseStrategy::Static,
            )
        }
    }
}
