use std::{borrow::Cow, path::Path};

use crate::config::package::PackageConfig;

/// Normalizes a path by replacing backslashes with forward slashes and removing
/// all "./" sequences. Uses Cow to avoid allocation when path is already
/// normalized.
///
/// On Unix systems with clean paths, this returns Cow::Borrowed
/// (zero allocation). Only allocates when normalization is actually needed.
pub fn normalize_path(path: &str) -> Cow<'_, str> {
    // Check if normalization is actually needed
    if path.contains('\\') || path.contains("./") {
        // Need to normalize - replaces ALL occurrences
        Cow::Owned(path.replace("\\", "/").replace("./", ""))
    } else {
        // Already normalized (no backslashes, no ./ sequences)
        Cow::Borrowed(path)
    }
}

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

    #[test]
    fn normalize_path_returns_borrowed_for_clean_unix_paths() {
        let path = "src/main.rs";
        let result = normalize_path(path);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "src/main.rs");
    }

    #[test]
    fn normalize_path_normalizes_windows_paths() {
        let path = "src\\main.rs";
        let result = normalize_path(path);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result, "src/main.rs");
    }

    #[test]
    fn normalize_path_removes_dot_slash_at_start() {
        let path = "./src/main.rs";
        let result = normalize_path(path);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result, "src/main.rs");
    }

    #[test]
    fn normalize_path_removes_dot_slash_in_middle() {
        let path = "packages/./api/src/main.rs";
        let result = normalize_path(path);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result, "packages/api/src/main.rs");
    }

    #[test]
    fn normalize_path_handles_multiple_issues() {
        let path = ".\\packages\\.\\api";
        let result = normalize_path(path);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result, "packages/api");
    }
}
