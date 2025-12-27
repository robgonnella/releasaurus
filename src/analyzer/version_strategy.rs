//! Version strategy trait and implementations for calculating next versions.
//!
//! This module provides a trait-based approach to version calculation,
//! allowing different strategies for stable releases, versioned prereleases,
//! and static prereleases.

use log::*;
use next_version::VersionUpdater;
use semver::Version;

use crate::{
    Result,
    analyzer::{helpers, release::Tag},
    config::prerelease::{PrereleaseConfig, PrereleaseStrategy},
};

/// Context for version calculation containing all necessary information.
#[derive(Debug, Clone)]
pub struct VersionContext<'a> {
    /// Current version tag (None for first release)
    pub current_tag: Option<&'a Tag>,
    /// Commit messages to analyze
    pub commits: &'a [String],
    /// Whether breaking changes should always increment major version
    pub breaking_always_increment_major: bool,
    /// Whether features should always increment minor version
    pub features_always_increment_minor: bool,
    /// Custom regex for major version increments
    pub custom_major_increment_regex: Option<&'a str>,
    /// Custom regex for minor version increments
    pub custom_minor_increment_regex: Option<&'a str>,
}

impl<'a> VersionContext<'a> {
    /// Create a VersionUpdater configured with the context's settings.
    pub fn create_version_updater(&self) -> Result<VersionUpdater> {
        let mut version_updater = VersionUpdater::new()
            .with_breaking_always_increment_major(
                self.breaking_always_increment_major,
            )
            .with_features_always_increment_minor(
                self.features_always_increment_minor,
            );

        if let Some(regex) = self.custom_major_increment_regex {
            version_updater =
                version_updater.with_custom_major_increment_regex(regex)?;
        }

        if let Some(regex) = self.custom_minor_increment_regex {
            version_updater =
                version_updater.with_custom_minor_increment_regex(regex)?;
        }

        Ok(version_updater)
    }
}

/// Trait for calculating the next version based on commits and current state.
pub trait VersionStrategy {
    /// Calculate the next version given the current context.
    fn calculate_next_version(
        &self,
        context: &VersionContext,
    ) -> Result<Version>;
}

/// Strategy for stable (non-prerelease) versions.
pub struct StableVersionStrategy;

impl StableVersionStrategy {
    /// Create a new stable version strategy.
    pub fn new() -> Self {
        Self
    }
}

impl VersionStrategy for StableVersionStrategy {
    fn calculate_next_version(
        &self,
        context: &VersionContext,
    ) -> Result<Version> {
        if let Some(current) = context.current_tag {
            if current.semver.pre.is_empty() {
                // Normal stable version bump
                debug!(
                    "stable version strategy: performing standard version update"
                );
                let version_updater = context.create_version_updater()?;
                Ok(version_updater
                    .increment(&current.semver, context.commits.to_vec()))
            } else {
                // Graduate from prerelease to stable
                info!(
                    "stable version strategy: graduating prerelease {} to stable",
                    current.semver
                );
                Ok(helpers::graduate_prerelease(&current.semver))
            }
        } else {
            // First release
            debug!("stable version strategy: first release");
            Ok(Version::parse("0.1.0")?)
        }
    }
}

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
    fn calculate_next_version(
        &self,
        context: &VersionContext,
    ) -> Result<Version> {
        if let Some(current) = context.current_tag {
            if current.semver.pre.is_empty() {
                // Starting new prerelease from stable
                info!(
                    "versioned prerelease strategy: starting new prerelease from stable {}",
                    current.semver
                );
                let version_updater = context.create_version_updater()?;
                let next_stable = version_updater
                    .increment(&current.semver, context.commits.to_vec());
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
                    info!(
                        "versioned prerelease strategy: incrementing prerelease {}",
                        current.semver
                    );
                    let version_updater = context.create_version_updater()?;
                    Ok(version_updater
                        .increment(&current.semver, context.commits.to_vec()))
                } else {
                    // Different prerelease identifier - switch to new one
                    info!(
                        "versioned prerelease strategy: switching from {} to {}",
                        current_pre_id, self.identifier
                    );
                    let stable_current =
                        helpers::graduate_prerelease(&current.semver);
                    let version_updater = context.create_version_updater()?;
                    let stable_next = version_updater
                        .increment(&stable_current, context.commits.to_vec());
                    helpers::add_prerelease(
                        stable_next,
                        &self.identifier,
                        PrereleaseStrategy::Versioned,
                    )
                }
            }
        } else {
            // First release as prerelease
            info!("versioned prerelease strategy: first release as prerelease");
            let version = Version::parse("0.1.0")?;
            helpers::add_prerelease(
                version,
                &self.identifier,
                PrereleaseStrategy::Versioned,
            )
        }
    }
}

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
    fn calculate_next_version(
        &self,
        context: &VersionContext,
    ) -> Result<Version> {
        if let Some(current) = context.current_tag {
            if current.semver.pre.is_empty() {
                // Starting new prerelease from stable
                info!(
                    "static prerelease strategy: starting new prerelease from stable {}",
                    current.semver
                );
                let version_updater = context.create_version_updater()?;
                let next_stable = version_updater
                    .increment(&current.semver, context.commits.to_vec());
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
                    info!(
                        "static prerelease strategy: incrementing prerelease {}",
                        current.semver
                    );
                    let mut version =
                        helpers::graduate_prerelease(&current.semver);
                    let version_updater = context.create_version_updater()?;
                    version = version_updater
                        .increment(&version, context.commits.to_vec());
                    helpers::add_prerelease(
                        version,
                        &self.identifier,
                        PrereleaseStrategy::Static,
                    )
                } else {
                    // Different prerelease identifier - switch to new one
                    info!(
                        "static prerelease strategy: switching from {} to {}",
                        current_pre_id, self.identifier
                    );
                    let stable_current =
                        helpers::graduate_prerelease(&current.semver);
                    let version_updater = context.create_version_updater()?;
                    let stable_next = version_updater
                        .increment(&stable_current, context.commits.to_vec());
                    helpers::add_prerelease(
                        stable_next,
                        &self.identifier,
                        PrereleaseStrategy::Static,
                    )
                }
            }
        } else {
            // First release as prerelease
            info!("static prerelease strategy: first release as prerelease");
            let version = Version::parse("0.1.0")?;
            helpers::add_prerelease(
                version,
                &self.identifier,
                PrereleaseStrategy::Static,
            )
        }
    }
}

/// Factory for creating version strategies based on configuration.
pub struct VersionStrategyFactory;

impl VersionStrategyFactory {
    /// Create a version strategy based on the provided prerelease configuration.
    pub fn create(
        prerelease: Option<&PrereleaseConfig>,
    ) -> Result<Box<dyn VersionStrategy>> {
        if let Some(config) = prerelease {
            let identifier = config.suffix()?.to_string();

            match config.strategy {
                PrereleaseStrategy::Versioned => {
                    Ok(Box::new(VersionedPrereleaseStrategy::new(identifier)))
                }
                PrereleaseStrategy::Static => {
                    Ok(Box::new(StaticPrereleaseStrategy::new(identifier)))
                }
            }
        } else {
            Ok(Box::new(StableVersionStrategy::new()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::release::Tag;

    fn create_test_tag(version: &str) -> Tag {
        Tag {
            name: version.to_string(),
            semver: Version::parse(version).unwrap(),
            sha: "abc123".to_string(),
            timestamp: None,
        }
    }

    fn create_basic_context<'a>(
        current_tag: Option<&'a Tag>,
        commits: &'a [String],
    ) -> VersionContext<'a> {
        VersionContext {
            current_tag,
            commits,
            breaking_always_increment_major: true,
            features_always_increment_minor: true,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
        }
    }

    #[test]
    fn test_stable_strategy_first_release() {
        let strategy = StableVersionStrategy::new();
        let commits = vec![];
        let context = create_basic_context(None, &commits);

        let result = strategy.calculate_next_version(&context).unwrap();
        assert_eq!(result, Version::parse("0.1.0").unwrap());
    }

    #[test]
    fn test_stable_strategy_increment_patch() {
        let strategy = StableVersionStrategy::new();
        let tag = create_test_tag("1.0.0");
        let commits = vec!["fix: bug fix".to_string()];
        let context = create_basic_context(Some(&tag), &commits);

        let result = strategy.calculate_next_version(&context).unwrap();
        assert_eq!(result, Version::parse("1.0.1").unwrap());
    }

    #[test]
    fn test_stable_strategy_graduate_prerelease() {
        let strategy = StableVersionStrategy::new();
        let tag = create_test_tag("1.0.0-alpha.1");
        let commits = vec![];
        let context = create_basic_context(Some(&tag), &commits);

        let result = strategy.calculate_next_version(&context).unwrap();
        assert_eq!(result, Version::parse("1.0.0").unwrap());
    }

    #[test]
    fn test_versioned_prerelease_first_release() {
        let strategy = VersionedPrereleaseStrategy::new("alpha".to_string());
        let commits = vec![];
        let context = create_basic_context(None, &commits);

        let result = strategy.calculate_next_version(&context).unwrap();
        assert_eq!(result, Version::parse("0.1.0-alpha.1").unwrap());
    }

    #[test]
    fn test_versioned_prerelease_from_stable() {
        let strategy = VersionedPrereleaseStrategy::new("alpha".to_string());
        let tag = create_test_tag("1.0.0");
        let commits = vec!["feat: new feature".to_string()];
        let context = create_basic_context(Some(&tag), &commits);

        let result = strategy.calculate_next_version(&context).unwrap();
        assert_eq!(result, Version::parse("1.1.0-alpha.1").unwrap());
    }

    #[test]
    fn test_versioned_prerelease_increment() {
        let strategy = VersionedPrereleaseStrategy::new("alpha".to_string());
        let tag = create_test_tag("1.0.0-alpha.1");
        let commits = vec!["fix: bug fix".to_string()];
        let context = create_basic_context(Some(&tag), &commits);

        let result = strategy.calculate_next_version(&context).unwrap();
        assert_eq!(result, Version::parse("1.0.0-alpha.2").unwrap());
    }

    #[test]
    fn test_versioned_prerelease_switch_identifier() {
        let strategy = VersionedPrereleaseStrategy::new("beta".to_string());
        let tag = create_test_tag("1.0.0-alpha.3");
        let commits = vec!["feat: new feature".to_string()];
        let context = create_basic_context(Some(&tag), &commits);

        let result = strategy.calculate_next_version(&context).unwrap();
        assert_eq!(result, Version::parse("1.1.0-beta.1").unwrap());
    }

    #[test]
    fn test_static_prerelease_first_release() {
        let strategy = StaticPrereleaseStrategy::new("SNAPSHOT".to_string());
        let commits = vec![];
        let context = create_basic_context(None, &commits);

        let result = strategy.calculate_next_version(&context).unwrap();
        assert_eq!(result, Version::parse("0.1.0-SNAPSHOT").unwrap());
    }

    #[test]
    fn test_static_prerelease_from_stable() {
        let strategy = StaticPrereleaseStrategy::new("dev".to_string());
        let tag = create_test_tag("1.0.0");
        let commits = vec!["feat: new feature".to_string()];
        let context = create_basic_context(Some(&tag), &commits);

        let result = strategy.calculate_next_version(&context).unwrap();
        assert_eq!(result, Version::parse("1.1.0-dev").unwrap());
    }

    #[test]
    fn test_static_prerelease_increment() {
        let strategy = StaticPrereleaseStrategy::new("SNAPSHOT".to_string());
        let tag = create_test_tag("1.0.0-SNAPSHOT");
        let commits = vec!["fix: bug fix".to_string()];
        let context = create_basic_context(Some(&tag), &commits);

        let result = strategy.calculate_next_version(&context).unwrap();
        assert_eq!(result, Version::parse("1.0.1-SNAPSHOT").unwrap());
    }

    #[test]
    fn test_factory_creates_stable_strategy() {
        let strategy = VersionStrategyFactory::create(None).unwrap();
        let commits = vec![];
        let context = create_basic_context(None, &commits);

        let result = strategy.calculate_next_version(&context).unwrap();
        assert_eq!(result, Version::parse("0.1.0").unwrap());
    }

    #[test]
    fn test_factory_creates_versioned_prerelease_strategy() {
        let config = PrereleaseConfig {
            suffix: Some("alpha".to_string()),
            strategy: PrereleaseStrategy::Versioned,
        };
        let strategy = VersionStrategyFactory::create(Some(&config)).unwrap();
        let commits = vec![];
        let context = create_basic_context(None, &commits);

        let result = strategy.calculate_next_version(&context).unwrap();
        assert_eq!(result, Version::parse("0.1.0-alpha.1").unwrap());
    }

    #[test]
    fn test_factory_creates_static_prerelease_strategy() {
        let config = PrereleaseConfig {
            suffix: Some("SNAPSHOT".to_string()),
            strategy: PrereleaseStrategy::Static,
        };
        let strategy = VersionStrategyFactory::create(Some(&config)).unwrap();
        let commits = vec![];
        let context = create_basic_context(None, &commits);

        let result = strategy.calculate_next_version(&context).unwrap();
        assert_eq!(result, Version::parse("0.1.0-SNAPSHOT").unwrap());
    }
}
