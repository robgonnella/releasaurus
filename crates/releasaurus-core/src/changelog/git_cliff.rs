//! A git-cliff implementation of a changelog [`Generator`]
use color_eyre::eyre::{Result, eyre};
use glob::Pattern;
use indexmap::IndexMap;
use log::*;
use regex::{Regex, RegexBuilder};
use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::{
    changelog::traits::{Generator, Output, Writer},
    config::SinglePackageConfig,
};

fn process_package_path(package_path: String) -> Result<Vec<Pattern>> {
    let path = Path::new(package_path.as_str());

    if !path.is_dir() {
        return Err(eyre!(
            "package path is not a valid directory: {}",
            path.to_string_lossy()
        ));
    }

    let mut package_path = package_path;

    // include paths only work on with globs
    if package_path.ends_with("*") {
        // if the path provided ends in * return as is
        debug!("using package path {package_path}")
    } else if package_path.ends_with("/") {
        // otherwise if ends in "/" return modified with glob
        package_path = format!("{package_path}**/*").to_string();
        debug!("modified package_path to include glob: {package_path}")
    } else {
        // otherwise return a modified version that adds /**/*
        package_path = format!("{package_path}/**/*").to_string();
        debug!("modified package_path to include directory glob {package_path}")
    };

    // return vec of glob Pattern or None
    let pattern = Pattern::new(&package_path)?;

    Ok(vec![pattern])
}

/// Represents a git-cliff implementation of [`Generator`], [`CurrentVersion`],
/// and [`NextVersion`]
pub struct GitCliffChangelog {
    config: Box<git_cliff_core::config::Config>,
    repo: git_cliff_core::repo::Repository,
    path: String,
}

impl GitCliffChangelog {
    /// Returns new instance based on provided configs
    pub fn new(config: SinglePackageConfig) -> Result<Self> {
        let mut cliff_config = git_cliff_core::embed::EmbeddedConfig::parse()?;

        cliff_config.changelog.body = config.changelog.body.clone();
        cliff_config.changelog.header = config.changelog.header.clone();
        cliff_config.changelog.footer = config.changelog.footer.clone();
        cliff_config.changelog.trim = true;
        cliff_config.git.conventional_commits = true;
        cliff_config.git.filter_unconventional = false;
        cliff_config.git.protect_breaking_commits = true;
        cliff_config.git.require_conventional = false;

        if let Some(remote) = config.github {
            cliff_config.remote.github.owner = remote.owner;
            cliff_config.remote.github.repo = remote.repo;
            cliff_config.remote.github.token = Some(remote.token.clone());
        } else if let Some(remote) = config.gitlab {
            cliff_config.remote.gitlab.owner = remote.owner;
            cliff_config.remote.gitlab.repo = remote.repo;
            cliff_config.remote.gitlab.token = Some(remote.token.clone());
        } else if let Some(remote) = config.gitea {
            cliff_config.remote.gitea.owner = remote.owner;
            cliff_config.remote.gitea.repo = remote.repo;
            cliff_config.remote.gitea.token = Some(remote.token.clone());
        } else if let Some(remote) = config.bitbucket {
            cliff_config.remote.bitbucket.owner = remote.owner;
            cliff_config.remote.bitbucket.repo = remote.repo;
            cliff_config.remote.bitbucket.token = Some(remote.token.clone());
        }

        let mut tag_prefix = "v".to_string();
        if !config.package.name.is_empty() {
            tag_prefix = format!("{}-v", config.package.name);
        }
        if let Some(prefix) = config.package.tag_prefix.clone() {
            tag_prefix = prefix
        }

        let re = Regex::new(&tag_prefix)?;
        cliff_config.git.tag_pattern = Some(re);
        cliff_config.bump.initial_tag = Some(format!("{}0.1.0", tag_prefix));

        cliff_config.git.include_paths =
            process_package_path(config.package.path.clone())?;

        let group_number_re = Regex::new(r"\d{1,2}\s-")?;
        let mut group_id = 0;

        for parser in cliff_config.git.commit_parsers.iter_mut() {
            if let Some(ref group) = parser.group
                && group_number_re.is_match(group)
            {
                group_id += 1;
                let new_group =
                    group_number_re.replace(group, format!("{group_id} -"));
                parser.group = Some(new_group.to_string());
            }
        }

        let mut breaking_change_parser =
            git_cliff_core::config::CommitParser::default();

        let br_message_re = Regex::new(r"^.+!:")?;
        let br_footer_re = RegexBuilder::new(r"breaking-?change:")
            .case_insensitive(true)
            .build()?;

        breaking_change_parser.message = Some(br_message_re);
        breaking_change_parser.footer = Some(br_footer_re);
        breaking_change_parser.group =
            Some("<!-- 0 -->❌ Breaking Changes".to_string());

        cliff_config
            .git
            .commit_parsers
            .insert(0, breaking_change_parser);

        let repo = git_cliff_core::repo::Repository::init(PathBuf::from("."))?;

        Ok(Self {
            config: Box::new(cliff_config),
            repo,
            path: config.package.path,
        })
    }

    fn process_releases<'a>(
        &self,
        commits: Vec<git2::Commit>,
        tags: IndexMap<String, git_cliff_core::tag::Tag>,
    ) -> Result<(Vec<git_cliff_core::release::Release<'a>>, Option<String>)>
    {
        let repository_path =
            self.repo.root_path()?.to_string_lossy().into_owned();

        // fill out and append to list of releases as we process commits
        let mut releases = vec![git_cliff_core::release::Release::default()];
        // track last "completed" release - meaning we found a tag
        let mut previous_release = git_cliff_core::release::Release::default();
        // keep track of the current version as we process commits / tags
        let mut current_version: Option<String> = None;

        // loop commits in reverse oldest -> newest
        for git_commit in commits.iter().rev() {
            // get release at end of list
            let release = releases.last_mut().unwrap();
            // copy commit
            let commit = git_cliff_core::commit::Commit::from(git_commit);
            let commit_id = commit.id.to_string();
            // add commit to release
            release.commits.push(commit);
            release.repository = Some(repository_path.clone());
            // set release commit - this will keep getting updated until we
            // get to the last commit in the release, which will be a tag
            release.commit_id = Some(commit_id);
            // now check if we have a tag matching this commit
            if let Some(tag) = tags.get(release.commit_id.as_ref().unwrap()) {
                // we only care about releases for this specific package
                if let Some(re) = self.config.git.tag_pattern.clone()
                    && !re.is_match(&tag.name)
                {
                    continue;
                }
                // we've found the top of the release!
                current_version = Some(tag.name.clone());
                release.version = Some(tag.name.to_string());
                release.message.clone_from(&tag.message);
                release.timestamp = Some(git_commit.time().seconds());

                // reset and get ready to process next release
                previous_release.previous = None;
                release.previous = Some(Box::new(previous_release));
                // set previous_release to release we just finished processing
                previous_release = release.clone();
                // add a new empty release to the end of our list so our loop
                // starts working on next release
                releases.push(git_cliff_core::release::Release::default());
            }
        }

        if let Some(rel) = releases.last()
            && rel.previous.is_none()
        {
            debug!("setting final release.previous");
            previous_release.previous = None;
            releases.last_mut().unwrap().previous =
                Some(Box::new(previous_release));
        }

        // set the commit range on each release
        for release in &mut releases {
            // Set the commit ranges for all releases
            if !release.commits.is_empty() {
                release.commit_range = Some(git_cliff_core::commit::Range::new(
                    release.commits.first().unwrap(),
                    release.commits.last().unwrap(),
                ))
            }
        }

        Ok((releases, current_version))
    }

    fn get_repo_releases<'a>(
        &self,
    ) -> Result<(Vec<git_cliff_core::release::Release<'a>>, Option<String>)>
    {
        let tags = self.repo.tags(&None, false, false)?;

        // get just the commits for the path specified or all commits
        // if option is None
        let commits = self.repo.commits(
            None,
            Some(self.config.git.include_paths.clone()),
            None,
            false,
        )?;

        // process and return the releases for this repo
        self.process_releases(commits, tags)
    }

    fn next_is_breaking(
        &self,
        current_version: Option<String>,
        next_version: Option<String>,
    ) -> Result<bool> {
        if next_version.is_none() {
            return Ok(false);
        }

        if current_version.is_none() {
            let mut next = next_version.unwrap();

            if let Some(pattern) = self.config.git.tag_pattern.clone() {
                next = pattern.replace(next.as_str(), "").into_owned();
            }

            let next_semver = semver::Version::parse(next.as_str()).unwrap();
            // 1st release don't consider it a breaking change unless
            // major version is at least 1
            return Ok(next_semver.major > 0);
        }

        let mut current = current_version.unwrap();
        let mut next = next_version.unwrap();

        if let Some(pattern) = self.config.git.tag_pattern.clone() {
            current = pattern.replace(current.as_str(), "").into_owned();
            next = pattern.replace(next.as_str(), "").into_owned();
        }

        let current_semver = semver::Version::parse(current.as_str())?;
        let next_semver = semver::Version::parse(next.as_str())?;

        debug!("comparing current {current} and next {next}");

        Ok(next_semver.major > current_semver.major)
    }
}

impl Generator for GitCliffChangelog {
    fn generate(&self) -> Result<Output> {
        let (releases, current_version) = self.get_repo_releases()?;

        let mut changelog = git_cliff_core::changelog::Changelog::new(
            releases,
            &self.config,
            None,
        )?;

        debug!("changelog: {:#?}", changelog);

        // increase to next version
        let next_version = changelog.bump_version()?;
        let is_breaking = self
            .next_is_breaking(current_version.clone(), next_version.clone())?;

        // generate content
        let mut buf = BufWriter::new(Vec::new());
        changelog.generate(&mut buf)?;
        let bytes = buf.into_inner()?;
        let out = String::from_utf8(bytes)?;

        Ok(Output {
            log: out,
            current_version,
            next_version,
            is_breaking,
        })
    }
}

impl Writer for GitCliffChangelog {
    fn write(&self) -> Result<Output> {
        let output = self.generate()?;
        let package_dir = Path::new(self.path.as_str());
        let file_path = package_dir.join("CHANGELOG.md");

        // OpenOptions allows fine-grained control over how a file is opened.
        let mut file = OpenOptions::new()
            .write(true) // Enable writing to the file
            .create(true) // Create the file if it doesn't exist
            .truncate(true) // Truncate the file to 0 length if it already exists
            .open(file_path)?;

        file.write_all(output.log.as_bytes())?;

        Ok(output)
    }
}

#[cfg(test)]
#[path = "./git_cliff_tests.rs"]
mod tests;
