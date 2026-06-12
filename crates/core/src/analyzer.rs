//! Commit analysis, version detection, and changelog generation.
//!
//! Parses conventional commits, determines semantic version bumps,
//! and generates formatted changelogs using Tera templates.

use crate::{
    analyzer::{
        config::AnalyzerConfig,
        group::GroupParser,
        release::Release,
        version_strategy::{context::Context, factory::VersionStrategyFactory},
    },
    forge::request::{ForgeCommit, Tag},
    result::Result,
};

pub mod commit;
pub mod config;
pub mod group;
mod helpers;
pub mod release;
mod version_strategy;

/// Analyzes commits using conventional commit patterns to determine version
/// bumps and generate changelogs.
pub struct Analyzer<'a> {
    config: &'a AnalyzerConfig,
    group_parser: GroupParser,
}

impl<'a> Analyzer<'a> {
    /// Create analyzer with changelog template configuration and tag prefix
    /// settings.
    pub fn new(config: &'a config::AnalyzerConfig) -> Result<Self> {
        Ok(Self {
            config,
            group_parser: GroupParser::default(),
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

        // calculate next release
        let releasable =
            self.process_release(&mut release, current_tag.as_ref())?;

        if !releasable {
            return Ok(None);
        }

        Ok(Some(release))
    }

    fn process_release(
        &self,
        release: &mut Release,
        current_tag: Option<&Tag>,
    ) -> Result<bool> {
        if release.commits.is_empty() {
            return Ok(false);
        }

        // Create version strategy based on configuration
        let strategy = VersionStrategyFactory::create(self.config)?;

        let commits: Vec<String> = release
            .commits
            .iter()
            .map(|c| c.raw_message.clone())
            .collect();

        let context = Context {
            current_tag,
            commits: &commits,
            config: self.config,
            short_sha: &release.short_sha,
            timestamp: release.timestamp,
        };

        let next = strategy.calculate_next_version(&context)?;

        if let Some(current) = current_tag
            && next == current.semver
        {
            return Ok(false);
        }

        let mut next_tag_name = next.to_string();

        if let Some(prefix) = self.config.tag_prefix.as_ref() {
            next_tag_name = format!("{prefix}{}", next);
        }

        let next_tag = Tag {
            name: next_tag_name,
            semver: next,
            // timestamp and sha are unknown until release-pr is merged
            timestamp: None,
            // SHA will be set when the release PR merges and creates a commit
            sha: "".into(),
        };

        if let Some(base_url) = self.config.release_link_base_url.as_ref() {
            release.link = base_url.join(&next_tag.name)?.to_string();
        }

        if let Some(base_url) = self.config.compare_link_base_url.as_ref()
            && let Some(current) = current_tag
        {
            release.tag_compare_link = base_url
                .join(&format!("{}...{}", current.name, next_tag.name))?
                .to_string();

            release.sha_compare_link = base_url
                .join(&format!("{}...{}", current.name, release.sha))?
                .to_string();
        }

        release.tag = next_tag;

        let context = tera::Context::from_serialize(&release)?;
        let notes = tera::Tera::one_off(&self.config.body, &context, false)?;
        release.notes = helpers::strip_extra_lines(notes.trim());

        Ok(true)
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

        // commits are ordered newest-first; the release sha and timestamp
        // come from the newest commit in the range regardless of any
        // conventional-commit filtering below, so compare links span the
        // entire range
        if let Some(newest) = commits.first() {
            release.sha = newest.id.clone();
            release.short_sha = newest.short_id.clone();
            release.timestamp = newest.timestamp;
        }

        for forge_commit in commits.iter() {
            if self
                .config
                .commit_modifiers
                .skip_shas
                .iter()
                .any(|sha| forge_commit.id.starts_with(sha))
            {
                log::debug!(
                    "skip_shas contains commit it: skipping {}",
                    forge_commit.id
                );
                continue;
            }

            let forge_commit = if let Some(reworded) = self
                .config
                .commit_modifiers
                .reword
                .iter()
                .find(|r| forge_commit.id.starts_with(&r.sha))
            {
                log::debug!("rewording commit: {}", forge_commit.id);
                &ForgeCommit {
                    message: reworded.message.clone(),
                    ..forge_commit.clone()
                }
            } else {
                forge_commit
            };

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
