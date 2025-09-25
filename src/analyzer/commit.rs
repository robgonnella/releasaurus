use git_conventional::Commit as ConventionalCommit;
use serde::Serialize;

use crate::{
    analyzer::group::{Group, GroupParser},
    forge::request::ForgeCommit,
};

/// Parsed commit with conventional commit information and metadata.
#[derive(Debug, Clone, Serialize)]
pub struct Commit {
    pub id: String,
    pub group: Group,
    pub scope: Option<String>,
    pub message: String,
    pub body: Option<String>,
    pub link: String,
    pub breaking: bool,
    pub breaking_description: Option<String>,
    pub merge_commit: bool,
    pub timestamp: i64,
    pub author_name: String,
    pub author_email: String,
    pub raw_message: String,
}

impl Commit {
    /// Parse git2 commit into structured commit with conventional commit parsing.
    pub fn parse_forge_commit(
        group_parser: &GroupParser,
        forge_commit: &ForgeCommit,
    ) -> Self {
        let author_name = forge_commit.author_name.clone();
        let author_email = forge_commit.author_email.clone();
        let commit_id = forge_commit.id.clone();
        let merge_commit = forge_commit.merge_commit;
        let raw_message = forge_commit.message.clone();
        let timestamp = forge_commit.timestamp;
        let parsed = ConventionalCommit::parse(raw_message.trim_end());
        let link = forge_commit.link.clone();

        match parsed {
            Ok(cc) => {
                let mut commit = Self {
                    id: commit_id,
                    scope: cc.scope().map(|s| s.to_string()),
                    message: cc.description().to_string(),
                    body: cc.body().map(|b| b.to_string()),
                    merge_commit,
                    breaking: cc.breaking(),
                    breaking_description: cc
                        .breaking_description()
                        .map(|d| d.to_string()),
                    raw_message: raw_message.to_string(),
                    group: Group::default(),
                    link,
                    timestamp,
                    author_name,
                    author_email,
                };
                commit.group = group_parser.parse(&commit);
                commit
            }
            Err(_) => {
                let split = raw_message
                    .split_once("\n")
                    .map(|(m, b)| (m.to_string(), b.to_string()));

                let (message, body) = match split {
                    Some((m, b)) => {
                        if b.is_empty() {
                            (m, None)
                        } else {
                            (m, Some(b))
                        }
                    }
                    None => ("".to_string(), None),
                };

                Self {
                    id: commit_id,
                    scope: None,
                    message,
                    body,
                    merge_commit,
                    breaking: false,
                    breaking_description: None,
                    raw_message: raw_message.to_string(),
                    group: Group::default(),
                    link,
                    timestamp,
                    author_name,
                    author_email,
                }
            }
        }
    }
}
