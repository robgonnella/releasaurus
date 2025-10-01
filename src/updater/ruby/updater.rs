use async_trait::async_trait;
use log::*;

use crate::{
    forge::{request::FileChange, traits::FileLoader},
    result::Result,
    updater::{framework::Package, traits::PackageUpdater},
};

/// Ruby package updater for Gem and Bundler projects.
pub struct RubyUpdater {}

impl RubyUpdater {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PackageUpdater for RubyUpdater {
    async fn update(
        &self,
        _packages: Vec<Package>,
        _loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        warn!("ruby updater not implemented yet");
        Ok(None)
    }
}
