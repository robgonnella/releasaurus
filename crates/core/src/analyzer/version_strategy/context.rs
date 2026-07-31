use next_version::VersionUpdater;
use semver::{BuildMetadata, Version};

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
    config::{
        prerelease::PrereleaseStrategy,
        versioning::{
            DEFAULT_BREAKING_ALWAYS_INCREMENT_MAJOR,
            DEFAULT_FEAT_ALWAYS_INCREMENT_MINOR,
        },
    },
    forge::request::Tag,
    result::Result,
};

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
    /// Builds the [`VersionUpdater`] the semantic strategies increment with.
    ///
    /// The increment flags reach here as `Option<bool>` - the resolver passes
    /// the config tiers through without applying a default - so this is the
    /// one place the defaults are applied. They come from the same constants
    /// the JSON schema documents, so the published default and the real
    /// behavior cannot drift apart.
    pub fn create_version_updater(&self) -> Result<VersionUpdater> {
        let mut version_updater = VersionUpdater::new()
            .with_breaking_always_increment_major(
                self.config
                    .breaking_always_increment_major
                    .unwrap_or(DEFAULT_BREAKING_ALWAYS_INCREMENT_MAJOR),
            )
            .with_features_always_increment_minor(
                self.config
                    .features_always_increment_minor
                    .unwrap_or(DEFAULT_FEAT_ALWAYS_INCREMENT_MINOR),
            );

        if let Some(regex) = self.config.custom_major_increment_regex.as_ref() {
            version_updater = version_updater
                .with_custom_major_increment_regex(regex.as_str())?;
        }

        if let Some(regex) = self.config.custom_minor_increment_regex.as_ref() {
            version_updater = version_updater
                .with_custom_minor_increment_regex(regex.as_str())?;
        }

        Ok(version_updater)
    }

    /// Next `major.minor.patch` version, with build metadata cleared.
    ///
    /// Build metadata belongs to the strategy that asked for it, never to
    /// the previous tag: both [`VersionUpdater::increment`] and
    /// [`helpers::graduate_prerelease`] carry the current version's metadata
    /// forward, so without this a tag left over from
    /// `major.minor.patch+timestamp.sha` would pin its timestamp and sha onto
    /// every later `major.minor.patch` release.
    /// [`SemanticBuildVersionStrategy`][super::semantic_build] assigns fresh
    /// metadata after calling this, so it is unaffected.
    pub fn get_next_semantic_version(&self) -> Result<Version> {
        let mut version = self.next_semantic_version_from_tag()?;
        version.build = BuildMetadata::EMPTY;
        Ok(version)
    }

    fn next_semantic_version_from_tag(&self) -> Result<Version> {
        if let Some(prerelease_config) = self.config.prerelease.as_ref() {
            let identifier = prerelease_config.suffix.clone();

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
