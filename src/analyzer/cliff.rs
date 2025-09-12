//! A git-cliff implementation of a changelog
use color_eyre::eyre::ContextCompat;
use indexmap::IndexMap;
use log::*;
use regex::Regex;
use std::{
    fs::OpenOptions,
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use crate::{
    analyzer::{
        cliff_helpers,
        config::AnalyzerConfig,
        types::{Output, ProjectedRelease, Version},
    },
    repo::StartingPoint,
    result::Result,
};

/// Represents a git-cliff implementation of a repository analyzer
pub struct CliffAnalyzer {
    analyzer_config: AnalyzerConfig,
    cliff_config: Box<git_cliff_core::config::Config>,
    repo: git_cliff_core::repo::Repository,
    package_full_path: PathBuf,
    starting_point: Option<StartingPoint>,
    commit_link_base_url: String,
    release_link_base_url: String,
}

impl CliffAnalyzer {
    /// Returns new instance based on provided configs
    pub fn new(config: AnalyzerConfig) -> Result<Self> {
        let repo_path = Path::new(&config.repo_path).to_path_buf();
        let package_full_path = repo_path.join(&config.package_relative_path);
        let starting_point = config.starting_point.clone();
        let commit_link_base_url = config.commit_link_base_url.clone();
        let release_link_base_url = config.release_link_base_url.clone();
        let cliff_config = cliff_helpers::get_cliff_config(config.clone())?;
        let repo = git_cliff_core::repo::Repository::init(repo_path)?;

        Ok(Self {
            analyzer_config: config,
            cliff_config: Box::new(cliff_config),
            repo,
            package_full_path,
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
                    self.cliff_config.git.tag_pattern.clone(),
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
            self.repo
                .tags(&self.cliff_config.git.tag_pattern, false, false)?;

        // use the parent of last release as starting point
        let start = self
            .starting_point
            .clone()
            .map(|c| format!("{}..HEAD", c.tagged_parent));

        let range = start.as_deref();

        info!("using range for commits: {:#?}", range);

        // get just the commits for the path specified or all commits
        // if option is None
        let commits = self.repo.commits(
            range,
            Some(self.cliff_config.git.include_paths.clone()),
            None,
            false,
        )?;

        // process and return the releases for this repo
        self.process_commits(commits, tags)
    }

    fn get_notes_for_latest_release(&self, changelog: &str) -> String {
        let stripped: Vec<&str> =
            cliff_helpers::BODY_END_REGEX.splitn(changelog, 2).collect();

        let notes = stripped[0].to_string();

        let header_re = Regex::new(
            format!(r"(?ms)(?<header>.*{})", cliff_helpers::HEADER_END_TAG)
                .as_str(),
        );

        if let Ok(rgx) = header_re
            && let Some(captures) = rgx.captures(changelog)
            && let Some(_header_value) = captures.name("header")
        {
            return rgx.replace_all(&notes, "").to_string();
        }

        notes
    }

    pub fn process_repository(&self) -> Result<Output> {
        info!(
            "processing repository for package: {}",
            self.package_full_path.display()
        );

        let (releases, current_version) = self.get_repo_releases()?;

        let mut current: Option<Version> = None;
        if let Some(tag) = current_version
            && let Some(pattern) = self.cliff_config.git.tag_pattern.clone()
        {
            let stripped = pattern.replace(&tag, "").to_string();
            let semver_version = semver::Version::parse(&stripped)?;
            current = Some(Version {
                tag,
                semver: semver_version,
            })
        }

        let mut changelog = git_cliff_core::changelog::Changelog::new(
            releases,
            &self.cliff_config,
            None,
        )?;

        // increase to next version
        let next_version = changelog.bump_version()?;

        // generate content
        let mut buf = BufWriter::new(Vec::new());
        changelog.generate(&mut buf)?;
        let bytes = buf.into_inner()?;
        let mut out = String::from_utf8(bytes)?;

        if self.starting_point.is_some() {
            // removes trailing release artifact that resulted from using
            // the parent commit of the previous release tag during commit
            // analysis
            out = self.get_notes_for_latest_release(&out);
        }

        let mut next: Option<Version> = None;

        if let Some(tag) = next_version.clone()
            && let Some(pattern) = self.cliff_config.git.tag_pattern.clone()
        {
            let stripped = pattern.replace(&tag, "").to_string();
            let semver_version = semver::Version::parse(&stripped)?;
            next = Some(Version {
                tag,
                semver: semver_version,
            })
        }

        let mut projected_release = None;

        if next_version.is_some() {
            let notes = self.get_notes_for_latest_release(&out);
            let last_release =
                changelog.releases.last().wrap_err("no releases found")?;
            projected_release = Some(ProjectedRelease {
                tag: next_version.clone().unwrap_or("".into()),
                path: self.package_full_path.display().to_string(),
                sha: last_release.commit_id.clone().unwrap_or("".into()),
                notes,
            });
        }

        Ok(Output {
            changelog: out,
            current_version: current,
            next_version: next,
            projected_release,
        })
    }

    pub fn write_changelog(&self) -> Result<Output> {
        let output = self.process_repository()?;
        let file_path = self.package_full_path.join("CHANGELOG.md");

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

        let mut content = format!("{}{}", output.changelog, existing_content);

        content = cliff_helpers::replace_header(
            &content,
            self.analyzer_config.header.clone(),
        );

        content = cliff_helpers::replace_footer(
            &content,
            self.analyzer_config.footer.clone(),
        );

        content = cliff_helpers::strip_internal_body_markers(&content);
        println!("------before------");
        println!("{content}");
        content = cliff_helpers::strip_extra_lines(&content);
        println!("------after------");
        println!("{content}");

        file.write_all(content.as_bytes())?;

        Ok(output)
    }
}

#[cfg(test)]
#[path = "./cliff_tests.rs"]
mod tests;
