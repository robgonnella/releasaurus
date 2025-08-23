use color_eyre::eyre::Result;

use crate::forge::types::{
    CreatePrRequest, GetPrRequest, PrLabelsRequest, UpdatePrRequest,
};

pub trait Forge {
    fn get_pr_number(&self, req: GetPrRequest) -> Result<Option<u64>>;
    fn create_pr(&self, req: CreatePrRequest) -> Result<u64>;
    fn update_pr(&self, req: UpdatePrRequest) -> Result<()>;
    fn add_pr_labels(&self, req: PrLabelsRequest) -> Result<()>;
    fn remove_pr_labels(&self, req: PrLabelsRequest) -> Result<()>;
}
