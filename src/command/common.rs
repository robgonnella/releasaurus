//! Common functionality shared between release commands
use std::path::Path;

use crate::config;

/// Get tag prefix for package, defaults to "v" or "{basename}-v".
pub fn get_tag_prefix(package: &config::PackageConfig) -> String {
    let mut default_for_package = "v".to_string();
    let package_path = Path::new(&package.path);
    if let Some(basename) = package_path.file_name() {
        default_for_package = format!("{}-v", basename.display());
    }
    package.tag_prefix.clone().unwrap_or(default_for_package)
}
