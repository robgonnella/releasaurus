use next_version::VersionUpdater;
use semver::Version;

use crate::{
    analyzer::{
        config::AnalyzerConfig,
        helpers,
        version_strategy::{
            prerelease_static::StaticPrereleaseStrategy,
            prerelease_versioned::VersionedPrereleaseStrategy,
            traits::VersionStrategy,
        },
    },
    config::prerelease::PrereleaseStrategy,
    forge::request::Tag,
    result::Result,
};

/// Default applied when the breaking/features increment flags are left unset.
const DEFAULT_INCREMENT_FLAG: bool = true;

/// Context for version calculation containing all necessary information.
#[derive(Debug)]
pub struct Context<'a> {
    /// Analyzer config
    pub config: &'a AnalyzerConfig,
    /// Current version tag (None for first release)
    pub current_tag: Option<&'a Tag>,
    /// Commit messages to analyze
    pub commits: &'a [String],
    /// Short sha for tip of release
    pub short_sha: &'a str,
    /// Timestamp for release
    pub timestamp: i64,
}

impl<'a> Context<'a> {
    pub fn create_version_updater(&self) -> Result<VersionUpdater> {
        let mut version_updater = VersionUpdater::new()
            .with_breaking_always_increment_major(
                self.config
                    .breaking_always_increment_major
                    .unwrap_or(DEFAULT_INCREMENT_FLAG),
            )
            .with_features_always_increment_minor(
                self.config
                    .features_always_increment_minor
                    .unwrap_or(DEFAULT_INCREMENT_FLAG),
            );

        if let Some(regex) = self.config.custom_major_increment_regex.as_ref() {
            version_updater =
                version_updater.with_custom_major_increment_regex(regex)?;
        }

        if let Some(regex) = self.config.custom_minor_increment_regex.as_ref() {
            version_updater =
                version_updater.with_custom_minor_increment_regex(regex)?;
        }

        Ok(version_updater)
    }

    pub fn get_next_semantic_version(&self) -> Result<Version> {
        if let Some(prerelease_config) = self.config.prerelease.as_ref() {
            let identifier = prerelease_config.suffix()?.to_string();

            match prerelease_config.strategy {
                PrereleaseStrategy::Versioned => {
                    VersionedPrereleaseStrategy::new(identifier)
                        .calculate_next_version(self)
                }
                PrereleaseStrategy::Static => {
                    StaticPrereleaseStrategy::new(identifier)
                        .calculate_next_version(self)
                }
            }
        } else if let Some(current) = self.current_tag {
            if current.semver.pre.is_empty() {
                // Normal stable version bump
                log::debug!(
                    "semantic version strategy: performing standard version update"
                );
                let version_updater = self.create_version_updater()?;
                Ok(version_updater.increment(&current.semver, self.commits))
            } else {
                // Graduate from prerelease to stable
                log::info!(
                    "semantic version strategy: graduating prerelease {} to stable",
                    current.semver
                );
                Ok(helpers::graduate_prerelease(&current.semver))
            }
        } else {
            // First release
            log::debug!("semantic version strategy: first release");
            Ok(Version::parse("0.1.0")?)
        }
    }
}
