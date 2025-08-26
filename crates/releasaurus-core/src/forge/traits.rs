use std::any::Any;

use color_eyre::eyre::Result;

use crate::forge::{
    config::RemoteConfig,
    types::{CreatePrRequest, GetPrRequest, PrLabelsRequest, UpdatePrRequest},
};

pub trait Forge: Any {
    fn config(&self) -> &RemoteConfig;
    fn get_pr_number(&self, req: GetPrRequest) -> Result<Option<u64>>;
    fn create_pr(&self, req: CreatePrRequest) -> Result<u64>;
    fn update_pr(&self, req: UpdatePrRequest) -> Result<()>;
    fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()>;
}
