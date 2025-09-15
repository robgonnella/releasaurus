use git_conventional::Commit as ConventionalCommit;
use git2::Commit as Git2Commit;
use serde::Serialize;

use crate::analyzer::groups::{Group, GroupParser};

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
    pub raw_message: String,
}

impl Commit {
    /// Parse git2 commit into structured commit with conventional commit parsing.
    pub fn parse_git2_commit(
        group_parser: &GroupParser,
        link_base: &str,
        g2_commit: &Git2Commit,
    ) -> Self {
        let commit_id = g2_commit.id().to_string();
        let merge_commit = g2_commit.parent_count() > 1;
        let raw_message = g2_commit.message().unwrap_or("").trim_end();
        let timestamp = g2_commit.time().seconds();
        let parsed = ConventionalCommit::parse(raw_message.trim_end());
        let link = format!("{}/{}", link_base, commit_id);

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
                }
            }
        }
    }
}
