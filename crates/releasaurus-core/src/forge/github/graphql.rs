use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PageInfo {
    #[serde(rename = "endCursor")]
    pub end_cursor: Option<String>,
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
}

pub const SHA_DATE_QUERY: &str = r#"
query GetShaDate($owner: String!, $repo: String!, $sha: GitObjectID!) {
  repository(owner: $owner, name: $repo) {
    startCommit: object(oid: $sha) {
      ... on Commit {
        committedDate
      }
    }
  }
}"#;

#[derive(Debug, Deserialize)]
pub struct StartCommit {
    #[serde(rename = "committedDate")]
    pub committed_date: String,
}

#[derive(Debug, Deserialize)]
pub struct StartCommitRepo {
    #[serde(rename = "startCommit")]
    pub start_commit: StartCommit,
}

#[derive(Debug, Deserialize)]
pub struct StartCommitData {
    pub repository: StartCommitRepo,
}

#[derive(Debug, Deserialize)]
pub struct StartCommitResult {
    pub data: StartCommitData,
}

#[derive(Debug, Serialize)]
pub struct ShaDateQueryVariables {
    pub owner: String,
    pub repo: String,
    pub sha: String,
}

pub const TAG_SEARCH_QUERY: &str = r#"
query GetRepoTagsDescending(
    $owner: String!
    $repo: String!
    $first: Int
    $cursor: String
) {
    repository(owner: $owner, name: $repo) {
        refs(
            refPrefix: "refs/tags/"
            first: $first
            orderBy: { field: TAG_COMMIT_DATE, direction: DESC }
            after: $cursor
        ) {
            nodes {
                name
                target {
                    __typename
                    ... on Commit {
                        oid
                        committedDate
                    }
                    ... on Tag {
                        oid
                        target {
                            ... on Commit {
                                oid
                                committedDate
                            }
                        }
                    }
                }
            }
            pageInfo {
                endCursor
                hasNextPage
            }
        }
    }
}
"#;

#[derive(Debug, Deserialize)]
pub struct TagSearchCommitNestedTarget {
    pub oid: String,
    #[serde(rename = "committedDate")]
    pub committed_date: String,
}

#[derive(Debug, Deserialize)]
pub enum TagSearchTypeName {
    Commit,
    Tag,
}

#[derive(Debug, Deserialize)]
pub struct TagSearchCommitTarget {
    pub __typename: TagSearchTypeName,
    pub oid: String,
    #[serde(rename = "committedDate")]
    pub committed_date: Option<String>,
    pub target: Option<TagSearchCommitNestedTarget>,
}

#[derive(Debug, Deserialize)]
pub struct TagSearchNode {
    pub name: String,
    pub target: TagSearchCommitTarget,
}

#[derive(Debug, Deserialize)]
pub struct TagSearchRefs {
    pub nodes: Vec<TagSearchNode>,
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
pub struct TagSearchRepository {
    pub refs: TagSearchRefs,
}

#[derive(Debug, Deserialize)]
pub struct TagSearchData {
    pub repository: TagSearchRepository,
}

#[derive(Debug, Deserialize)]
pub struct TagSearchResult {
    pub data: TagSearchData,
}

#[derive(Debug, Serialize)]
pub struct TagSearchQueryVariables {
    pub owner: String,
    pub repo: String,
    pub first: usize,
    pub cursor: Option<String>,
}
