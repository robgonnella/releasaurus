use semver::Version;

use crate::{
    analyzer::{
        helpers,
        version_strategy::{context::Context, traits::VersionStrategy},
    },
    config::prerelease::PrereleaseStrategy,
    result::Result,
};

/// Strategy for versioned prerelease versions (e.g., alpha.1, alpha.2).
pub struct VersionedPrereleaseStrategy {
    identifier: String,
}

impl VersionedPrereleaseStrategy {
    /// Create a new versioned prerelease strategy with the given identifier.
    pub fn new(identifier: String) -> Self {
        Self { identifier }
    }
}

impl VersionStrategy for VersionedPrereleaseStrategy {
    fn calculate_next_version(&self, ctx: &Context) -> Result<Version> {
        if let Some(current) = ctx.current_tag {
            if current.semver.pre.is_empty() {
                // Starting new prerelease from stable
                log::info!(
                    "versioned prerelease strategy: starting new prerelease from stable {}",
                    current.semver
                );
                let version_updater = ctx.create_version_updater()?;
                let next_stable =
                    version_updater.increment(&current.semver, ctx.commits);
                helpers::add_prerelease(
                    next_stable,
                    &self.identifier,
                    PrereleaseStrategy::Versioned,
                )
            } else {
                // Currently in a prerelease
                let current_pre_id =
                    current.semver.pre.as_str().split('.').next().unwrap_or("");

                if current_pre_id == self.identifier {
                    // Same prerelease identifier - increment it
                    log::info!(
                        "versioned prerelease strategy: incrementing prerelease {}",
                        current.semver
                    );
                    let version_updater = ctx.create_version_updater()?;
                    Ok(version_updater.increment(&current.semver, ctx.commits))
                } else {
                    // Different prerelease identifier - switch to new one
                    log::info!(
                        "versioned prerelease strategy: switching from {} to {}",
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
                        PrereleaseStrategy::Versioned,
                    )
                }
            }
        } else {
            // First release as prerelease
            log::info!(
                "versioned prerelease strategy: first release as prerelease"
            );
            let version = Version::parse("0.1.0")?;
            helpers::add_prerelease(
                version,
                &self.identifier,
                PrereleaseStrategy::Versioned,
            )
        }
    }
}
