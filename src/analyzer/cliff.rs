//! A git-cliff implementation of a changelog [`Generator`]
use color_eyre::eyre::{ContextCompat, Result};
use indexmap::IndexMap;
use log::*;
use std::{
    fs::OpenOptions,
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use crate::analyzer::{
    cliff_helpers,
    config::AnalyzerConfig,
    types::{Output, ProjectedRelease},
};

/// Represents a git-cliff implementation of a repository analyzer
pub struct CliffAnalyzer {
    config: Box<git_cliff_core::config::Config>,
    repo: git_cliff_core::repo::Repository,
    path: String,
    starting_point: Option<(String, String)>,
    commit_link_base_url: String,
    release_link_base_url: String,
}

impl CliffAnalyzer {
    /// Returns new instance based on provided configs
    pub fn new(config: AnalyzerConfig) -> Result<Self> {
        let path = config.package_path.clone();
        let starting_point = config.starting_point.clone();
        let commit_link_base_url = config.commit_link_base_url.clone();
        let release_link_base_url = config.release_link_base_url.clone();
        let cliff_config = cliff_helpers::get_cliff_config(config)?;
        let repo = git_cliff_core::repo::Repository::init(PathBuf::from("."))?;

        Ok(Self {
            config: Box::new(cliff_config),
            repo,
            path,
            starting_point,
            commit_link_base_url,
            release_link_base_url,
        })
    }

    fn process_commits<'a>(
        &self,
        commits: Vec<git2::Commit>,
        tags: IndexMap<String, git_cliff_core::tag::Tag>,
    ) -> Result<(Vec<git_cliff_core::release::Release<'a>>, Option<String>)>
    {
        let repository_path =
            self.repo.root_path()?.to_string_lossy().into_owned();

        // fill out and append to list of releases as we process commits
        let mut releases = vec![git_cliff_core::release::Release::default()];
        // track last "completed" release - meaning we found a tag so we
        // can update release.previous where needed
        let mut previous_release = git_cliff_core::release::Release::default();
        // keep track of the current version as we process commits / tags
        let mut current_version: Option<String> = None;

        // loop commits in reverse oldest -> newest
        for git_commit in commits.iter().rev() {
            // skip release commit since it would have been the last release
            // commit which was already processed and added to changelog in
            // last release
            if let Some(starting_point) = self.starting_point.clone()
                && git_commit.id().to_string() == starting_point.0
            {
                continue;
            }

            // get release at end of list
            let release = releases.last_mut().unwrap();
            // add commit details to release
            cliff_helpers::update_release_with_commit(
                repository_path.clone(),
                release,
                git_commit,
            );
            // now check if we have a tag matching this commit
            if let Some(tag) = tags.get(release.commit_id.as_ref().unwrap())
                && let Some(version) = cliff_helpers::process_tag_for_release(
                    release,
                    git_commit,
                    tag,
                    self.config.git.tag_pattern.clone(),
                )
            {
                // reset and get ready to process next release by adding a new
                // "empty" release to the end of our list
                current_version = Some(version);
                previous_release.previous = None;
                release.previous = Some(Box::new(previous_release));
                previous_release = release.clone();
                releases.push(git_cliff_core::release::Release::default());
            }
        }

        // ensure last release in list has previous set
        if let Some(rel) = releases.last()
            && rel.previous.is_none()
        {
            previous_release.previous = None;
            releases.last_mut().unwrap().previous =
                Some(Box::new(previous_release));
        }

        // set the commit range and version link for each release
        for release in &mut releases {
            // add extra version_link property
            cliff_helpers::add_link_base_and_commit_range_to_release(
                release,
                &self.commit_link_base_url,
                &self.release_link_base_url,
            );
        }

        Ok((releases, current_version))
    }

    fn get_repo_releases<'a>(
        &self,
    ) -> Result<(Vec<git_cliff_core::release::Release<'a>>, Option<String>)>
    {
        let tags =
            self.repo.tags(&self.config.git.tag_pattern, false, false)?;

        // use the parent of last release as starting point
        // c.0 = release commit
        // c.1 = release commit parent
        let start = self
            .starting_point
            .clone()
            .map(|c| format!("{}..HEAD", c.1));

        let range = start.as_deref();

        info!("using range for commits: {:#?}", range);

        // get just the commits for the path specified or all commits
        // if option is None
        let commits = self.repo.commits(
            range,
            Some(self.config.git.include_paths.clone()),
            None,
            false,
        )?;

        // process and return the releases for this repo
        self.process_commits(commits, tags)
    }

    pub fn process_repository(&self) -> Result<Output> {
        info!("processing repository for package: {}", self.path);

        let (releases, current_version) = self.get_repo_releases()?;

        let mut changelog = git_cliff_core::changelog::Changelog::new(
            releases,
            &self.config,
            None,
        )?;

        // increase to next version
        let next_version = changelog.bump_version()?;

        // generate content
        let mut buf = BufWriter::new(Vec::new());
        changelog.generate(&mut buf)?;
        let bytes = buf.into_inner()?;
        let out = String::from_utf8(bytes)?;

        let mut projected_release = None;

        if next_version.is_some() {
            let notes = cliff_helpers::parse_projected_release_notes(&out);
            let last_release =
                changelog.releases.last().wrap_err("no releases found")?;
            projected_release = Some(ProjectedRelease {
                tag: next_version.clone().unwrap_or("".into()),
                path: self.path.clone(),
                sha: last_release.commit_id.clone().unwrap_or("".into()),
                notes,
            });
        }

        Ok(Output {
            changelog: out,
            current_version,
            next_version,
            projected_release,
        })
    }

    pub fn write_changelog(&self) -> Result<Output> {
        let output = self.process_repository()?;
        let package_dir = Path::new(self.path.as_str());
        let file_path = package_dir.join("CHANGELOG.md");

        let mut existing_content = String::from("");

        // if we're starting from a specific point in time we won't get
        // the entire changelog generated in output so we'll want to read
        // in existing content and prepend
        if self.starting_point.is_some() {
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

        let content = format!("{}{}", output.changelog, existing_content);

        println!("{content}");

        file.write_all(content.as_bytes())?;

        Ok(output)
    }
}

#[cfg(test)]
#[path = "./cliff_tests.rs"]
mod tests;
