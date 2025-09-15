//! A git-cliff implementation of a changelog
use glob::Pattern;
use log::*;
use next_version::VersionUpdater;
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::{
    analyzer::{
        config::AnalyzerConfig,
        groups::GroupParser,
        helpers,
        types::{Release, Tag},
    },
    repo::Repository,
    result::Result,
};

/// Repository analyzer for commit analysis and changelog generation.
pub struct Analyzer<'r> {
    config: AnalyzerConfig,
    repo: &'r Repository,
    include_paths: Vec<Pattern>,
    group_parser: GroupParser,
    package_full_path: PathBuf,
}

impl<'r> Analyzer<'r> {
    /// Create analyzer with configuration and repository.
    pub fn new(config: AnalyzerConfig, repo: &'r Repository) -> Result<Self> {
        let repo_path = Path::new(&config.repo_path).to_path_buf();
        let package_full_path = repo_path.join(&config.package_relative_path);
        let include_paths = helpers::process_package_path(
            &config.repo_path,
            &config.package_relative_path,
        )?;

        Ok(Self {
            config,
            include_paths,
            repo,
            group_parser: GroupParser::new(),
            package_full_path,
        })
    }

    /// Analyze commits and generate release information.
    pub fn process_repository(&self) -> Result<Option<Release>> {
        info!(
            "processing repository for package: {}",
            self.package_full_path.display()
        );

        let mut release = self.get_repo_release()?;

        if release.commits.is_empty() {
            return Ok(None);
        }

        let current_version = self.config.starting_tag.clone();

        if let Some(current) = current_version.clone() {
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

            let next_tag = Tag {
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
        }

        Ok(Some(release))
    }

    /// Generate and write changelog to CHANGELOG.md file.
    pub fn write_changelog(&self) -> Result<Option<Release>> {
        let release = self.process_repository()?;

        if release.is_none() {
            return Ok(None);
        }

        let release = release.unwrap();

        let file_path = self.package_full_path.join("CHANGELOG.md");

        let mut existing_content = String::from("");

        // if we're starting from a specific point in time we won't get
        // the entire changelog generated in output so we'll want to read
        // in existing content and prepend
        if self.config.starting_tag.is_some() {
            let mut read_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false) // don't truncate here so we can read content
                .open(file_path.clone())?;

            read_file.read_to_string(&mut existing_content)?;

            drop(read_file);
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path.clone())?;

        let mut content = format!("{}\n\n{}", release.notes, existing_content);

        content = helpers::replace_header(&content, self.config.header.clone());

        content = helpers::replace_footer(&content, self.config.footer.clone());

        content = helpers::strip_extra_lines(&content);

        file.write_all(content.as_bytes())?;

        Ok(Some(release))
    }

    /// Process commits and build release information.
    fn process_commits(&self, commits: Vec<git2::Commit>) -> Result<Release> {
        // fill out and append to list of releases as we process commits
        let mut release = Release::default();

        // loop commits in reverse oldest -> newest
        for git_commit in commits.iter().rev() {
            // add commit details to release
            helpers::update_release_with_commit(
                &self.group_parser,
                &self.config.commit_link_base_url,
                &mut release,
                git_commit,
            );
        }

        Ok(release)
    }

    /// Get commits since last release and process them.
    fn get_repo_release(&self) -> Result<Release> {
        // use the parent of last release as starting point
        let start = self
            .config
            .starting_tag
            .clone()
            .map(|c| format!("{}..HEAD", c.sha));

        let range = start.as_deref();

        info!("using range for commits: {:#?}", range);

        // get just the commits for the path specified or all commits
        // if option is None
        let commits =
            self.repo.commits(range, Some(self.include_paths.clone()))?;

        // process and return the releases for this repo
        self.process_commits(commits)
    }
}

#[cfg(test)]
#[path = "./changelog_tests.rs"]
mod tests;
