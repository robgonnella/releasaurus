use std::path::Path;

use crate::config::package::{PackageConfig, SubPackage};

/// Resolves the package name from config or derives from path.
///
/// If the package name is explicitly set in config, uses that.
/// Otherwise, derives the name from the last component of the
/// workspace_root + path combination.
pub fn resolve_package_name(
    package: &PackageConfig,
    repo_name: &str,
) -> String {
    if !package.name.is_empty() {
        return package.name.clone();
    }

    Path::new(&package.workspace_root)
        .join(&package.path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| repo_name.to_string())
}

/// Resolves the sub-package name from config or derives from path.
///
/// Similar to resolve_package_name but for sub-packages which use
/// the workspace_root as their base path.
pub fn resolve_sub_package_name(
    package: &SubPackage,
    workspace_root: &str,
    repo_name: &str,
) -> String {
    if !package.name.is_empty() {
        return package.name.clone();
    }

    Path::new(workspace_root)
        .join(&package.path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| repo_name.to_string())
}

#[cfg(test)]
mod tests {
    use crate::resolver::resolvers::test_helper::create_test_package;

    use super::*;

    #[test]
    fn resolve_package_name_uses_config_when_set() {
        let pkg = create_test_package("my-package");
        let name = resolve_package_name(&pkg, "repo");
        assert_eq!(name, "my-package");
    }

    #[test]
    fn resolve_package_name_derives_from_path() {
        let mut pkg = create_test_package("");
        pkg.path = "packages/api".to_string();
        let name = resolve_package_name(&pkg, "repo");
        assert_eq!(name, "api");
    }

    #[test]
    fn resolve_package_name_fallback_to_repo() {
        let pkg = create_test_package("");
        let name = resolve_package_name(&pkg, "fallback-repo");
        assert_eq!(name, "fallback-repo");
    }
}
