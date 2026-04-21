//! Path normalization utilities for cross-platform compatibility.
//!
//! These utilities ensure consistent path handling across Windows
//! and Unix systems by normalizing separators and removing redundant
//! path segments.

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use crate::config::package::PackageConfig;

/// Normalizes a path by replacing backslashes with forward slashes
/// and removing all "./" sequences.
///
/// Uses Cow to avoid allocation when path is already normalized.
/// On Unix systems with clean paths, this returns Cow::Borrowed
/// (zero allocation). Only allocates when normalization is needed.
///
/// # Examples
///
/// ```
/// # use std::borrow::Cow;
/// # fn normalize_path(path: &str) -> Cow<'_, str> {
/// #     if path.contains('\\') || path.contains("./") {
/// #         Cow::Owned(path.replace("\\", "/").replace("./", ""))
/// #     } else {
/// #         Cow::Borrowed(path)
/// #     }
/// # }
/// assert_eq!(normalize_path("src/main.rs"), "src/main.rs");
/// assert_eq!(normalize_path("src\\main.rs"), "src/main.rs");
/// assert_eq!(normalize_path("./src/main.rs"), "src/main.rs");
/// ```
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

/// Normalizes workspace and full package paths.
pub fn normalize_package_paths(package: &PackageConfig) -> (PathBuf, PathBuf) {
    let mut normalized_root = normalize_path(&package.workspace_root);
    if normalized_root == "." {
        normalized_root = Cow::from("");
    }

    let normalized_workspace_root =
        Path::new(normalized_root.as_ref()).to_path_buf();

    let full_path = normalized_workspace_root
        .join(&package.path)
        .to_string_lossy()
        .to_string();

    let mut normalized_full = normalize_path(&full_path);
    if normalized_full == "." {
        normalized_full = Cow::from("");
    }

    let normalized_full_path =
        Path::new(normalized_full.as_ref()).to_path_buf();

    (normalized_workspace_root, normalized_full_path)
}

/// Normalizes additional paths for a package.
pub fn normalize_additional_paths(package: &PackageConfig) -> Vec<PathBuf> {
    package
        .additional_paths
        .clone()
        .unwrap_or_default()
        .iter()
        .map(|p| {
            let normalized = normalize_path(p).to_string();
            if normalized == "." {
                Path::new("").to_path_buf()
            } else {
                Path::new(&normalized).to_path_buf()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_borrowed_for_clean_paths() {
        let path = "src/main.rs";
        let result = normalize_path(path);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "src/main.rs");
    }

    #[test]
    fn normalizes_windows_paths() {
        assert_eq!(normalize_path("src\\main.rs"), "src/main.rs");
    }

    #[test]
    fn removes_dot_slash_at_start() {
        assert_eq!(normalize_path("./src/main.rs"), "src/main.rs");
    }

    #[test]
    fn removes_dot_slash_in_middle() {
        assert_eq!(
            normalize_path("packages/./api/src/main.rs"),
            "packages/api/src/main.rs"
        );
    }

    #[test]
    fn handles_multiple_issues() {
        assert_eq!(normalize_path(".\\packages\\.\\api"), "packages/api");
    }

    #[test]
    fn handles_empty_path() {
        assert_eq!(normalize_path(""), "");
    }
}
