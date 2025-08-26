use std::path::Path;

use color_eyre::eyre::{Result, eyre};
use log::*;
use regex::Regex;
use reqwest::Url;
use tempfile::TempDir;

use crate::forge::config::RemoteConfig;

pub struct Git {
    pub tmp_dir: TempDir,
    pub default_branch: String,
    repo: git2::Repository,
}

impl Git {
    pub fn new(config: RemoteConfig) -> Result<Self> {
        let tmp = TempDir::new()?;

        let repo_url = format!(
            "{}://{}/{}/{}",
            config.scheme, config.host, config.owner, config.repo
        );

        let url = Url::parse(repo_url.as_str())?;

        let repo = git2::Repository::clone(url.as_str(), tmp.path())?;

        repo.remote_rename("origin", "upstream")?;

        let mut remote = repo.find_remote("upstream")?;
        remote.connect(git2::Direction::Fetch)?;

        let buf = remote.default_branch()?;
        let default_branch =
            buf.as_str().ok_or(eyre!("failed to get default branch"))?;
        let default_branch = default_branch.replace("refs/heads/", "");

        drop(remote);

        Ok(Self {
            tmp_dir: tmp,
            repo,
            default_branch,
        })
    }

    pub fn path(&self) -> &Path {
        self.tmp_dir.path()
    }

    pub fn get_latest_tag_commit(&self, prefix: &str) -> Option<String> {
        let prefix_rgx = format!(r"^{}", prefix);
        let re = Regex::new(&prefix_rgx).ok()?;
        let references = self.repo.references_glob("refs/tags/*").ok()?;
        let tags: Vec<git2::Reference> = references
            .filter_map(|r| r.ok())
            .filter(|r| {
                if let Some(name) = r.name()
                    && let Some(stripped) = name.strip_prefix("refs/tags/")
                {
                    info!("stripped --> {stripped}");
                    return re.is_match(stripped);
                }
                false
            })
            .collect();

        if let Some(tag) = tags.last() {
            let name = tag.name()?.strip_prefix("refs/tags/")?;
            return Some(name.to_string());
        }

        None
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

    pub fn commit(&self, msg: &str) -> Result<()> {
        let mut index = self.repo.index()?;
        let oid = index.write_tree()?;
        let tree = self.repo.find_tree(oid)?;
        let parent_commit = self.repo.head()?.peel_to_commit()?;
        let author = git2::Signature::now("Releasaurus", "rele@saurus.com")?;
        let committer = git2::Signature::now("Releasaurus", "rele@saurus.com")?;
        self.repo.commit(
            Some("HEAD"),
            &author,
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
