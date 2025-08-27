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
    pub body: String,
}

#[derive(Debug)]
pub struct PrLabelsRequest {
    pub pr_number: u64,
    pub labels: Vec<String>,
}
