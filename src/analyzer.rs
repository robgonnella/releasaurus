//! Commit analysis, version detection, and changelog generation.
//!
//! Parses conventional commits, determines semantic version bumps,
//! and generates formatted changelogs using Tera templates.

use log::*;
use next_version::VersionUpdater;
use semver::Version;

use crate::{
    Result,
    analyzer::release::{Release, Tag},
    config::prerelease::{PrereleaseConfig, PrereleaseStrategy},
    forge::request::ForgeCommit,
};

mod commit;
pub mod config;
mod group;
mod helpers;
pub mod release;

/// Analyzes commits using conventional commit patterns to determine version
/// bumps and generate changelogs.
pub struct Analyzer<'a> {
    config: &'a config::AnalyzerConfig,
    group_parser: group::GroupParser,
}

impl<'a> Analyzer<'a> {
    /// Create analyzer with changelog template configuration and tag prefix
    /// settings.
    pub fn new(config: &'a config::AnalyzerConfig) -> Result<Self> {
        Ok(Self {
            config,
            group_parser: group::GroupParser::new(),
        })
    }

    /// Analyze commits to calculate the next semantic version and generate
    /// formatted release notes using Tera templates.
    pub fn analyze(
        &self,
        commits: Vec<ForgeCommit>,
        current_tag: Option<Tag>,
    ) -> Result<Option<Release>> {
        let mut release = self.process_commits(commits)?;

        if release.commits.is_empty() {
            return Ok(None);
        }

        // calculate next release
        if let Some(current) = current_tag {
            self.process_release_from_current_tag(&mut release, &current)?;
        } else {
            self.process_first_release(&mut release)?;
        }

        Ok(Some(release))
    }

    fn process_first_release(&self, release: &mut Release) -> Result<()> {
        // this is the first release
        let mut semver = semver::Version::parse("0.1.0")?;

        // Handle prerelease for first release
        if let Some(prerelease) = &self.config.prerelease {
            semver = helpers::add_prerelease(
                semver,
                prerelease.suffix()?,
                prerelease.strategy,
            )?;
        }

        let mut tag_name = semver.to_string();
        if let Some(prefix) = self.config.tag_prefix.as_ref() {
            tag_name = format!("{prefix}{tag_name}");
        }
        let next_tag = release::Tag {
            sha: release.sha.clone(),
            name: tag_name,
            semver,
            timestamp: None,
        };

        release.link =
            format!("{}/{}", self.config.release_link_base_url, next_tag.name);

        release.tag = Some(next_tag);

        let context = tera::Context::from_serialize(&release)?;

        let notes = tera::Tera::one_off(&self.config.body, &context, false)?;

        release.notes = helpers::strip_extra_lines(notes.trim());

        Ok(())
    }

    fn process_release_from_current_tag(
        &self,
        release: &mut Release,
        current: &Tag,
    ) -> Result<()> {
        let commits = release
            .commits
            .iter()
            .map(|c| c.raw_message.to_string())
            .collect::<Vec<String>>();

        let mut version_updater = VersionUpdater::new()
            .with_breaking_always_increment_major(
                self.config.breaking_always_increment_major,
            )
            .with_features_always_increment_minor(
                self.config.features_always_increment_minor,
            );

        if let Some(value) = self.config.custom_major_increment_regex.as_ref() {
            version_updater =
                version_updater.with_custom_major_increment_regex(value)?;
        }

        if let Some(value) = self.config.custom_minor_increment_regex.as_ref() {
            version_updater =
                version_updater.with_custom_minor_increment_regex(value)?;
        }

        // Handle prerelease transitions
        let next = if let Some(prerelease) = &self.config.prerelease {
            info!(
                "calculating next prerelease version: suffix: {:?}, strategy: {:?}",
                prerelease.suffix, prerelease.strategy
            );

            // User wants a prerelease
            self.calculate_next_prerelease_version(
                current,
                &commits,
                prerelease,
                version_updater,
            )?
        } else {
            // No prerelease requested
            if current.semver.pre.is_empty() {
                // Normal stable version bump
                debug!(
                    "no prerelease configured - preforming standard version update"
                );
                version_updater.increment(&current.semver, commits)
            } else {
                info!(
                    "graduating current prerelease, {}, to stable version",
                    current.semver
                );
                // Graduate from prerelease to stable
                // Just remove prerelease suffix without bumping version
                helpers::graduate_prerelease(&current.semver)
            }
        };

        let mut next_tag_name = next.to_string();

        if let Some(prefix) = self.config.tag_prefix.as_ref() {
            next_tag_name = format!("{prefix}{}", next);
        }

        let next_tag = release::Tag {
            name: next_tag_name,
            semver: next,
            // we won't know timestamp or sha until release-pr is created
            timestamp: None,
            sha: "".into(),
        };

        release.link =
            format!("{}/{}", self.config.release_link_base_url, next_tag.name);

        release.tag = Some(next_tag);

        let context = tera::Context::from_serialize(&release)?;
        let notes = tera::Tera::one_off(&self.config.body, &context, false)?;
        release.notes = helpers::strip_extra_lines(notes.trim());
        Ok(())
    }

    fn calculate_next_prerelease_version(
        &self,
        current: &Tag,
        commits: &[String],
        prerelease: &PrereleaseConfig,
        version_updater: VersionUpdater,
    ) -> Result<Version> {
        // User wants a prerelease
        let prerelease_id = prerelease.suffix()?;
        if current.semver.pre.is_empty() {
            info!(
                "current version, {}, is not a prerelease: starting new prerelease",
                current.semver
            );

            // Currently stable, starting a prerelease
            let next_stable =
                version_updater.increment(&current.semver, commits);

            let version = helpers::add_prerelease(
                next_stable,
                prerelease_id,
                prerelease.strategy,
            )?;
            Ok(version)
        } else {
            // Currently in a prerelease
            let current_pre_id =
                current.semver.pre.as_str().split('.').next().unwrap_or("");
            if current_pre_id == prerelease_id {
                info!(
                    "current version, {}, is prerelease that matches config",
                    current.semver
                );
                match prerelease.strategy {
                    PrereleaseStrategy::Static => {
                        info!("preforming static prerelease increment");
                        // first graduate to remove suffix
                        let mut version =
                            helpers::graduate_prerelease(&current.semver);
                        // then increment version as normal
                        version = version_updater.increment(&version, commits);
                        // finally re-add the static prerelease suffix
                        Ok(helpers::add_prerelease(
                            version,
                            prerelease_id,
                            prerelease.strategy,
                        )?)
                    }
                    PrereleaseStrategy::Versioned => {
                        // Same prerelease identifier - increment it
                        info!("preforming versioned prerelease increment");
                        Ok(version_updater.increment(&current.semver, commits))
                    }
                }
            } else {
                info!(
                    "current tag has prerelease that does not match config - graduating prerelease and adding new prerelease suffix"
                );
                // Different prerelease identifier - switch to new one
                // Graduate to stable, calculate next version, then add new prerelease
                let stable_current =
                    helpers::graduate_prerelease(&current.semver);
                let stable_next =
                    version_updater.increment(&stable_current, commits);
                let version = helpers::add_prerelease(
                    stable_next,
                    prerelease_id,
                    prerelease.strategy,
                )?;
                Ok(version)
            }
        }
    }

    /// Parse commits into structured format with conventional commit
    /// categorization and grouping.
    fn process_commits(
        &self,
        commits: Vec<ForgeCommit>,
    ) -> Result<release::Release> {
        // fill out and append to list of releases as we process commits
        let mut release = release::Release::default();

        if self.config.include_author {
            release.include_author = true;
        }

        // loop commits in reverse oldest -> newest
        for forge_commit in commits.iter() {
            // add commit details to release
            helpers::update_release_with_commit(
                &self.group_parser,
                &mut release,
                forge_commit,
                self.config,
            );
        }

        Ok(release)
    }
}

#[cfg(test)]
mod tests;
