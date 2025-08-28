#[derive(Debug)]
pub struct Release {
    pub tag: String,
    pub notes: String,
}

#[derive(Debug)]
pub struct ReleasePullRequest {
    pub number: u64,
    pub sha: String,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub releases: Vec<Release>,
}

#[derive(Debug)]
pub struct GetPrRequest {
    pub head_branch: String,
    pub base_branch: String,
}

#[derive(Debug)]
pub struct CreatePrRequest {
    pub head_branch: String,
    pub base_branch: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug)]
pub struct UpdatePrRequest {
    pub pr_number: u64,
    pub title: String,
    pub body: String,
}

#[derive(Debug)]
pub struct PrLabelsRequest {
    pub pr_number: u64,
    pub labels: Vec<String>,
}
