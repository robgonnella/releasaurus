//! Common functionality shared between release commands
use log::*;
use std::path::Path;

use crate::{
    config,
    forge::{request::PrLabelsRequest, traits::Forge},
    result::Result,
};

/// Get tag prefix for package, defaults to "v" or "{basename}-v".
pub fn get_tag_prefix(package: &config::PackageConfig) -> String {
    let mut default_for_package = "v".to_string();
    let package_path = Path::new(&package.path);
    if let Some(basename) = package_path.file_name() {
        default_for_package = format!("{}-v", basename.display());
    }
    package.tag_prefix.clone().unwrap_or(default_for_package)
}

/// Log package processing info.
pub fn log_package_processing(package_path: &str, tag_prefix: &str) {
    info!(
        "processing changelog for package path: {}, tag_prefix: {}",
        package_path, tag_prefix
    );
}

/// Update PR labels via forge API.
pub async fn update_pr_labels(
    forge: &dyn Forge,
    pr_number: u64,
    labels: Vec<String>,
) -> Result<()> {
    let req = PrLabelsRequest { pr_number, labels };
    forge.replace_pr_labels(req).await
}
