//! Commit analysis, version detection, and changelog generation.
//!
//! Parses conventional commits, determines semantic version bumps,
//! and generates formatted changelogs using Tera templates.

use next_version::VersionUpdater;

use crate::{forge::request::ForgeCommit, result::Result};
mod commit;
pub mod config;
mod group;
mod helpers;
pub mod release;

/// Repository analyzer for commit analysis and changelog generation.
pub struct Analyzer {
    config: config::AnalyzerConfig,
    group_parser: group::GroupParser,
}

impl Analyzer {
    /// Create analyzer with configuration and repository.
    pub fn new(config: config::AnalyzerConfig) -> Result<Self> {
        Ok(Self {
            config,
            group_parser: group::GroupParser::new(),
        })
    }

    /// Analyze commits and generate release information.
    pub fn analyze(
        &self,
        commits: Vec<ForgeCommit>,
        current_tag: Option<release::Tag>,
    ) -> Result<Option<release::Release>> {
        let mut release = self.process_commits(commits)?;

        if release.commits.is_empty() {
            return Ok(None);
        }

        // calculate next release
        if let Some(current) = current_tag.clone() {
            let commits = release
                .commits
                .iter()
                .map(|c| c.raw_message.to_string())
                .collect::<Vec<String>>();

            let version_updater = VersionUpdater::new()
                .with_breaking_always_increment_major(true)
                .with_features_always_increment_minor(true);

            let next = version_updater.increment(&current.semver, commits);

            let mut next_tag_name = next.to_string();

            if let Some(prefix) = self.config.tag_prefix.clone() {
                next_tag_name = format!("{prefix}{}", next);
            }

            let next_tag = release::Tag {
                sha: release.sha.clone(),
                name: next_tag_name.clone(),
                semver: next,
            };

            release.link = format!(
                "{}/{}",
                self.config.release_link_base_url, next_tag.name
            );

            release.tag = Some(next_tag.clone());

            let context = tera::Context::from_serialize(&release)?;
            let notes =
                tera::Tera::one_off(&self.config.body, &context, false)?;
            release.notes = helpers::strip_extra_lines(notes.trim());
        } else {
            // this is the first release
            let mut tag_name = "0.1.0".to_string();
            let semver = semver::Version::parse(&tag_name).unwrap();
            if let Some(prefix) = self.config.tag_prefix.clone() {
                tag_name = format!("{prefix}{tag_name}");
            }
            let next_tag = release::Tag {
                sha: release.sha.clone(),
                name: tag_name,
                semver,
            };

            release.link = format!(
                "{}/{}",
                self.config.release_link_base_url, next_tag.name
            );

            release.tag = Some(next_tag.clone());

            let context = tera::Context::from_serialize(&release)?;

            let notes =
                tera::Tera::one_off(&self.config.body, &context, false)?;

            release.notes = helpers::strip_extra_lines(notes.trim());
        }

        Ok(Some(release))
    }

    /// Process commits and build release information.
    fn process_commits(
        &self,
        commits: Vec<ForgeCommit>,
    ) -> Result<release::Release> {
        // fill out and append to list of releases as we process commits
        let mut release = release::Release::default();

        // loop commits in reverse oldest -> newest
        for forge_commit in commits.iter() {
            // add commit details to release
            helpers::update_release_with_commit(
                &self.group_parser,
                &mut release,
                forge_commit,
            );
        }

        Ok(release)
    }
}
