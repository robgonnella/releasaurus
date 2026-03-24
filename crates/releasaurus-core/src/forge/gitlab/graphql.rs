use graphql_client::{GraphQLQuery, QueryBody};
use serde::{Deserialize, Serialize};

const COMMIT_DIFF_QUERY: &str = r#"
query GetCommitDiff($project_id: ID!, $commit_sha: String!) {
  project(fullPath: $project_id) {
    repository {
      commit(ref: $commit_sha) {
        diffs {
            newPath
            oldPath
        }
      }
    }
  }
}"#;

#[derive(Debug, Serialize)]
pub struct CommitDiffQueryVars {
    pub project_id: String,
    pub commit_sha: String,
}

#[derive(Debug, Deserialize)]
pub struct CommitFilenameDiff {
    #[serde(rename = "oldPath")]
    pub old_path: Option<String>,
    #[serde(rename = "newPath")]
    pub new_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommitDiffCommit {
    pub diffs: Vec<CommitFilenameDiff>,
}

#[derive(Debug, Deserialize)]
pub struct CommitDiffRepository {
    pub commit: CommitDiffCommit,
}

#[derive(Debug, Deserialize)]
pub struct CommitDiffProject {
    pub repository: CommitDiffRepository,
}

#[derive(Debug, Deserialize)]
pub struct CommitDiffResponse {
    pub project: CommitDiffProject,
}

pub struct CommitDiffQuery {}

impl GraphQLQuery for CommitDiffQuery {
    type ResponseData = CommitDiffResponse;
    type Variables = CommitDiffQueryVars;

    fn build_query(variables: Self::Variables) -> QueryBody<Self::Variables> {
        QueryBody {
            variables,
            query: COMMIT_DIFF_QUERY,
            operation_name: "GetCommitDiff",
        }
    }
}
