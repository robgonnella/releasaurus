/// Release pull request information.
#[derive(Debug, Clone)]
pub struct ReleasePullRequest {
    pub number: u64,
    pub sha: String,
}

/// Request to get pull request by branch names.
#[derive(Debug, Clone)]
pub struct GetPrRequest {
    pub head_branch: String,
    pub base_branch: String,
}

/// Request to create a new pull request.
#[derive(Debug, Clone)]
pub struct CreatePrRequest {
    pub head_branch: String,
    pub base_branch: String,
    pub title: String,
    pub body: String,
}

/// Request to update existing pull request.
#[derive(Debug, Clone)]
pub struct UpdatePrRequest {
    pub pr_number: u64,
    pub title: String,
    pub body: String,
}

/// Request to update pull request labels.
#[derive(Debug, Clone)]
pub struct PrLabelsRequest {
    pub pr_number: u64,
    pub labels: Vec<String>,
}

/// Represents a normalized commit returned from any forge
#[derive(Debug)]
pub struct ForgeCommit {
    pub id: String,
}
