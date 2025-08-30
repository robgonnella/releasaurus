//! Facilitates interaction with a local git repository
use color_eyre::eyre::{Result, eyre};
use git2::RemoteCallbacks;
use log::*;
use regex::Regex;
use reqwest::Url;
use secrecy::ExposeSecret;
use std::path::Path;

use crate::forge::config::RemoteConfig;

const DEFAULT_UPSTREAM_REMOTE: &str = "upstream";

pub struct Repository {
    pub default_branch: String,
    config: RemoteConfig,
    repo: git2::Repository,
}

fn get_auth_callbacks<'r>(user: String, token: String) -> RemoteCallbacks<'r> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(move |_url, _username, _allowed| {
        git2::Cred::userpass_plaintext(&user, &token)
    });
    callbacks
}

impl Repository {
    pub fn new(local_path: &Path, config: RemoteConfig) -> Result<Self> {
        let repo_url =
            format!("{}://{}/{}", config.scheme, config.host, config.path);

        let url = Url::parse(repo_url.as_str())?;
        let git_config = git2::Config::open_default()?.snapshot()?;
        let user = git_config.get_str("user.name")?;
        let token = config.token.expose_secret().to_string();

        // setup callbacks for authentication
        let callbacks = get_auth_callbacks(user.into(), token.clone());

        // Sets a maximum depth of 250 commits when cloning to prevent cloning
        // thousands of commits. This is one of the reasons this project works
        // best on repos that enforce linear commit histories. If we tried
        // a depth of 250 on something like torvalds/linux repo we would get
        // many thousands of commits due to the non-linear history of that repo
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.depth(250);
        fetch_options.remote_callbacks(callbacks);

        let mut builder = git2::build::RepoBuilder::new();
        let repo = builder
            .fetch_options(fetch_options)
            .clone(url.as_str(), local_path)?;

        repo.remote_rename("origin", DEFAULT_UPSTREAM_REMOTE)?;

        // setup callbacks for authentication
        let callbacks = get_auth_callbacks(user.into(), token.clone());
        let mut remote = repo.find_remote(DEFAULT_UPSTREAM_REMOTE)?;
        remote.connect_auth(git2::Direction::Fetch, Some(callbacks), None)?;

        let buf = remote.default_branch()?;
        let default_branch =
            buf.as_str().ok_or(eyre!("failed to get default branch"))?;
        let default_branch = default_branch.replace("refs/heads/", "");

        drop(remote);

        Ok(Self {
            repo,
            config,
            default_branch,
        })
    }

    pub fn get_latest_tagged_starting_point(
        &self,
        prefix: &str,
    ) -> Result<Option<String>> {
        let regex_prefix = format!(r"^{}", prefix);
        let tag_regex = Regex::new(&regex_prefix)?;
        let references = self
            .repo
            .references()?
            .filter_map(|r| r.ok())
            .collect::<Vec<git2::Reference>>();

        // Iterate through all references in the repository in reverse and stop
        // at first that matches prefix
        for reference in references.into_iter().rev() {
            // Check if the reference is a tag with desired prefix
            if reference.is_tag()
                && let Some(name) = reference.name()
                && let Some(stripped) = name.strip_prefix("refs/tags/")
                && tag_regex.is_match(stripped)
            {
                let commit = reference.peel_to_commit()?;

                // return the parent of the tagged commit so the commit range
                // is inclusive of the tagged commit
                if let Ok(parent) = commit.parent(0) {
                    return Ok(Some(parent.id().to_string()));
                }
            }
        }
        Ok(None)
    }

    pub fn create_branch(&self, branch: &str) -> Result<()> {
        info!("creating branch: {branch}");
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        self.repo.branch(branch, &commit, true)?;
        Ok(())
    }

    pub fn switch_branch(&self, branch: &str) -> Result<()> {
        info!("switching to branch: {branch}");
        let ref_name = format!("refs/heads/{}", branch);
        let target_obj = self.repo.revparse_single(&ref_name)?;
        self.repo.checkout_tree(&target_obj, None)?;
        self.repo.set_head(&ref_name)?;
        Ok(())
    }

    pub fn add_all(&self) -> Result<()> {
        debug!("adding changed files to index");
        let mut index = self.repo.index()?;
        index.add_all(["."], git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    pub fn commit(&self, msg: &str) -> Result<()> {
        debug!("committing changes with msg: {msg}");
        let config = self.repo.config()?.snapshot()?;
        let user = config.get_str("user.name")?;
        let email = config.get_str("user.email")?;
        debug!("using committer: user: {user}, email: {email}");
        let mut index = self.repo.index()?;
        let oid = index.write_tree()?;
        let tree = self.repo.find_tree(oid)?;
        let parent_commit = self.repo.head()?.peel_to_commit()?;
        let committer = git2::Signature::now(user, email)?;
        self.repo.commit(
            Some("HEAD"),
            &committer,
            &committer,
            msg,
            &tree,
            &[&parent_commit],
        )?;
        Ok(())
    }

    pub fn push_branch(&self, branch: &str) -> Result<()> {
        info!("pushing branch {branch}");
        // setup callbacks for authentication
        let config = self.repo.config()?.snapshot()?;
        let user = config.get_str("user.name")?;
        let token = self.config.token.expose_secret().to_string();
        let callbacks = get_auth_callbacks(user.into(), token.clone());
        let mut push_opts = git2::PushOptions::default();
        push_opts.remote_callbacks(callbacks);

        let mut remote = self.repo.find_remote(DEFAULT_UPSTREAM_REMOTE)?;

        // + indicates "force" push
        let ref_spec = format!("+refs/heads/{branch}");
        remote.push(&[ref_spec], Some(&mut push_opts))?;

        Ok(())
    }

    // pub fn tag_current_head(&self, tag: &str) -> Result<()> {
    //     let head = self.repo.head()?;
    //     let oid = head
    //         .target()
    //         .ok_or(eyre!("failed to find current head oid"))?;
    //     let commit = self.repo.find_commit(oid)?;
    //     let tagger = git2::Signature::now("Releasaurus", "rele@saurs.com")?;
    //     self.repo
    //         .tag(tag, commit.as_object(), &tagger, tag, false)?;
    //     Ok(())
    // }
}
