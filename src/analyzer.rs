//! Commit analysis, version detection, and changelog generation.
//!
//! Parses conventional commits, determines semantic version bumps,
//! and generates formatted changelogs using Tera templates.

use crate::{
    Result,
    analyzer::{
        release::{Release, Tag},
        version_strategy::{VersionContext, VersionStrategyFactory},
    },
    forge::request::ForgeCommit,
};

mod commit;
pub mod config;
mod group;
mod helpers;
pub mod release;
mod version_strategy;

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
        self.process_release(&mut release, current_tag.as_ref())?;

        Ok(Some(release))
    }

    fn process_release(
        &self,
        release: &mut Release,
        current_tag: Option<&Tag>,
    ) -> Result<()> {
        // Create version strategy based on configuration
        let strategy =
            VersionStrategyFactory::create(self.config.prerelease.as_ref())?;

        let commits = release
            .commits
            .iter()
            .map(|c| c.raw_message.to_string())
            .collect::<Vec<String>>();

        let context = VersionContext {
            current_tag,
            commits: &commits,
            breaking_always_increment_major: self
                .config
                .breaking_always_increment_major,
            features_always_increment_minor: self
                .config
                .features_always_increment_minor,
            custom_major_increment_regex: self
                .config
                .custom_major_increment_regex
                .as_deref(),
            custom_minor_increment_regex: self
                .config
                .custom_minor_increment_regex
                .as_deref(),
        };

        let next = strategy.calculate_next_version(&context)?;

        let mut next_tag_name = next.to_string();

        if let Some(prefix) = self.config.tag_prefix.as_ref() {
            next_tag_name = format!("{prefix}{}", next);
        }

        let next_tag = release::Tag {
            name: next_tag_name,
            semver: next,
            // timestamp and sha are unknown until release-pr is merged
            timestamp: None,
            // SHA will be set when the release PR merges and creates a commit
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
