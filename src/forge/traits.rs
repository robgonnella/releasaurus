//! Traits related to remote git forges
use std::any::Any;

use color_eyre::eyre::Result;

use crate::forge::{
    config::RemoteConfig,
    types::{
        CreatePrRequest, GetPrRequest, PrLabelsRequest, ReleasePullRequest,
        UpdatePrRequest,
    },
};

pub trait Forge: Any {
    fn config(&self) -> &RemoteConfig;
    fn get_open_release_pr(
        &self,
        req: GetPrRequest,
    ) -> Result<Option<ReleasePullRequest>>;
    fn create_pr(&self, req: CreatePrRequest) -> Result<ReleasePullRequest>;
    fn update_pr(&self, req: UpdatePrRequest) -> Result<()>;
    fn replace_pr_labels(&self, req: PrLabelsRequest) -> Result<()>;
}
