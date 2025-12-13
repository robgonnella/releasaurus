use std::path::Path;

use crate::config::package::PackageConfig;

pub fn package_path(package: &PackageConfig, file: Option<&str>) -> String {
    let mut pkg_path = Path::new(&package.workspace_root).join(&package.path);

    if let Some(file) = file {
        pkg_path = pkg_path.join(file);
    }

    pkg_path
        .display()
        .to_string()
        .replace("\\", "/")
        .replace("./", "")
}

pub fn workspace_path(package: &PackageConfig, file: Option<&str>) -> String {
    let mut wrkspc_path = Path::new(&package.workspace_root).to_path_buf();

    if let Some(file) = file {
        wrkspc_path = wrkspc_path.join(file);
    }

    wrkspc_path
        .display()
        .to_string()
        .replace("\\", "/")
        .replace("./", "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{package::PackageConfig, release_type::ReleaseType};

    fn create_test_package(workspace_root: &str, path: &str) -> PackageConfig {
        PackageConfig {
            name: "test-package".to_string(),
            workspace_root: workspace_root.to_string(),
            path: path.to_string(),
            release_type: Some(ReleaseType::Node),
            ..Default::default()
        }
    }

    #[test]
    fn package_path_returns_package_directory() {
        let pkg = create_test_package(".", "packages/my-app");
        let result = package_path(&pkg, None);
        assert_eq!(result, "packages/my-app");
    }

    #[test]
    fn package_path_joins_file_to_package_directory() {
        let pkg = create_test_package(".", "packages/my-app");
        let result = package_path(&pkg, Some("package.json"));
        assert_eq!(result, "packages/my-app/package.json");
    }

    #[test]
    fn package_path_strips_leading_dot_slash() {
        let pkg = create_test_package(".", ".");
        let result = package_path(&pkg, Some("package.json"));
        assert_eq!(result, "package.json");
    }

    #[test]
    fn workspace_path_returns_workspace_root() {
        let pkg = create_test_package(".", "packages/my-app");
        let result = workspace_path(&pkg, None);
        assert_eq!(result, ".");
    }

    #[test]
    fn workspace_path_joins_file_to_workspace_root() {
        let pkg = create_test_package(".", "packages/my-app");
        let result = workspace_path(&pkg, Some("Cargo.lock"));
        assert_eq!(result, "Cargo.lock");
    }

    #[test]
    fn workspace_path_handles_nested_workspace_root() {
        let pkg = create_test_package("workspace", "packages/my-app");
        let result = workspace_path(&pkg, Some("Cargo.lock"));
        assert_eq!(result, "workspace/Cargo.lock");
    }
}
