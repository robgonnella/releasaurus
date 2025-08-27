use color_eyre::eyre::{Result, eyre};
use log::*;
use reqwest::Url;
use std::path::Path;

use crate::forge::config::RemoteConfig;

pub struct Git {
    pub default_branch: String,
    repo: git2::Repository,
}

impl Git {
    pub fn new(local_path: &Path, config: RemoteConfig) -> Result<Self> {
        let repo_url = format!(
            "{}://{}/{}/{}",
            config.scheme, config.host, config.owner, config.repo
        );

        let url = Url::parse(repo_url.as_str())?;

        // Sets a maximum depth of 250 commits when cloning to prevent cloning
        // thousands of commits. This is one of the reasons this project works
        // best on repos that enforce linear commit histories. If we tried
        // a depth of 250 on something like torvalds/linux repo we would get
        // many thousands of commits due to the non-linear history of that repo
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.depth(250);

        let mut builder = git2::build::RepoBuilder::new();
        let repo = builder
            .fetch_options(fetch_options)
            .clone(url.as_str(), local_path)?;

        repo.remote_rename("origin", "upstream")?;

        let mut remote = repo.find_remote("upstream")?;
        remote.connect(git2::Direction::Fetch)?;

        let buf = remote.default_branch()?;
        let default_branch =
            buf.as_str().ok_or(eyre!("failed to get default branch"))?;
        let default_branch = default_branch.replace("refs/heads/", "");

        drop(remote);

        Ok(Self {
            repo,
            default_branch,
        })
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
        let mut index = self.repo.index()?;
        index.add_all(["."], git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    pub fn commit(&self, msg: &str, author: &str, email: &str) -> Result<()> {
        let mut index = self.repo.index()?;
        let oid = index.write_tree()?;
        let tree = self.repo.find_tree(oid)?;
        let parent_commit = self.repo.head()?.peel_to_commit()?;
        let committer = git2::Signature::now(author, email)?;
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

    pub fn tag_current_head(&self, tag: &str) -> Result<()> {
        let head = self.repo.head()?;
        let oid = head
            .target()
            .ok_or(eyre!("failed to find current head oid"))?;
        let commit = self.repo.find_commit(oid)?;
        let tagger = git2::Signature::now("Releasaurus", "rele@saurs.com")?;
        self.repo
            .tag(tag, commit.as_object(), &tagger, tag, false)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {}
