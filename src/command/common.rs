//! Common functionality shared between release commands
use std::path::Path;

use crate::{
    analyzer::config::AnalyzerConfig,
    config::{self, Config},
    forge::config::RemoteConfig,
};

/// Resolve tag prefix for package from config or generate default based on
/// package path for monorepo support.
pub fn get_tag_prefix(package: &config::PackageConfig) -> String {
    let mut default_for_package = "v".to_string();
    let package_path = Path::new(&package.path);
    if let Some(basename) = package_path.file_name() {
        default_for_package = format!("{}-v", basename.display());
    }
    package.tag_prefix.clone().unwrap_or(default_for_package)
}

/// Generates [`AnalyzerConfig`] from [`Config`], [`RemoteConfig`],
/// and tag_prefix [`String`]
pub fn generate_analyzer_config(
    config: &Config,
    remote_config: &RemoteConfig,
    tag_prefix: String,
) -> AnalyzerConfig {
    AnalyzerConfig {
        body: config.changelog.body.clone(),
        include_author: config.changelog.include_author,
        skip_chore: config.changelog.skip_chore,
        skip_ci: config.changelog.skip_ci,
        skip_miscellaneous: config.changelog.skip_miscellaneous,
        release_link_base_url: remote_config.release_link_base_url.clone(),
        tag_prefix: Some(tag_prefix),
    }
}
