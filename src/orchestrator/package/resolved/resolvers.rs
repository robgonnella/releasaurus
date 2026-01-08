//! Resolution functions for package configuration.
//!
//! These functions handle the complex logic of resolving package
//! settings from multiple sources: global config, package config,
//! and CLI overrides.

use std::{collections::HashMap, path::Path};

use crate::{
    Result,
    cli::{GlobalOverrides, PackageOverrides},
    config::{
        package::{DEFAULT_TAG_PREFIX, PackageConfig, SubPackage},
        prerelease::PrereleaseConfig,
    },
};

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

/// Resolves the tag prefix for a package.
///
/// Logic:
/// - If explicitly set in config, use that
/// - If package is not at root, use `{package_name}-v`
/// - Otherwise use default `v`
pub fn resolve_tag_prefix(package: &PackageConfig) -> String {
    if let Some(prefix) = package.tag_prefix.as_ref() {
        return prefix.clone();
    }

    if package.workspace_root != "." || package.path != "." {
        format!("{}-v", package.name)
    } else {
        DEFAULT_TAG_PREFIX.to_string()
    }
}

/// Resolves whether auto-start-next is enabled.
///
/// Precedence: package config > global config > false (default)
pub fn resolve_auto_start_next(
    package: &PackageConfig,
    global_auto_start_next: Option<bool>,
) -> bool {
    package
        .auto_start_next
        .or(global_auto_start_next)
        .unwrap_or_default()
}

/// Resolves prerelease configuration with complex override logic.
///
/// Precedence (highest to lowest):
/// 1. Package-level CLI overrides
/// 2. Global CLI overrides
/// 3. Package-level config
/// 4. Global config
///
/// Returns None if no suffix is set after all resolution.
pub fn resolve_prerelease(
    package: &PackageConfig,
    global_prerelease: &PrereleaseConfig,
    global_overrides: &GlobalOverrides,
    package_overrides: &HashMap<String, PackageOverrides>,
) -> Result<Option<PrereleaseConfig>> {
    let mut prerelease = global_prerelease.clone();

    // Package config overrides global config
    if let Some(pkg_prerelease) = package.prerelease.clone() {
        prerelease = pkg_prerelease;
    }

    // Global CLI overrides override config
    if let Some(ref suffix) = global_overrides.prerelease_suffix {
        prerelease.suffix = Some(suffix.clone());
    }
    if let Some(strategy) = global_overrides.prerelease_strategy {
        prerelease.strategy = strategy;
    }

    // Package-level CLI overrides override everything
    if let Some(overrides) = package_overrides.get(&package.name) {
        if let Some(ref suffix) = overrides.prerelease_suffix {
            prerelease.suffix = Some(suffix.clone());
        }
        if let Some(strategy) = overrides.prerelease_strategy {
            prerelease.strategy = strategy;
        }
    }

    // Clean and validate suffix
    prerelease.suffix = prerelease
        .suffix
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if prerelease.suffix.is_some() {
        Ok(Some(prerelease))
    } else {
        Ok(None)
    }
}

/// Resolves version increment flags (breaking/features).
///
/// Precedence: package config > global config
pub fn resolve_version_increment_flags(
    package: &PackageConfig,
    global_breaking: bool,
    global_features: bool,
) -> (bool, bool) {
    let breaking = package
        .breaking_always_increment_major
        .unwrap_or(global_breaking);
    let features = package
        .features_always_increment_minor
        .unwrap_or(global_features);
    (breaking, features)
}

/// Resolves custom increment regex patterns.
///
/// Precedence: package config > global config
pub fn resolve_custom_increment_regexes(
    package: &PackageConfig,
    global_custom_major: &Option<String>,
    global_custom_minor: &Option<String>,
) -> (Option<String>, Option<String>) {
    let major = package
        .custom_major_increment_regex
        .clone()
        .or_else(|| global_custom_major.clone());
    let minor = package
        .custom_minor_increment_regex
        .clone()
        .or_else(|| global_custom_minor.clone());
    (major, minor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::prerelease::PrereleaseStrategy;

    fn create_test_package(name: &str) -> PackageConfig {
        PackageConfig {
            name: name.to_string(),
            workspace_root: ".".to_string(),
            path: ".".to_string(),
            ..Default::default()
        }
    }

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

    #[test]
    fn resolve_tag_prefix_uses_explicit_config() {
        let mut pkg = create_test_package("test");
        pkg.tag_prefix = Some("custom-v".to_string());
        assert_eq!(resolve_tag_prefix(&pkg), "custom-v");
    }

    #[test]
    fn resolve_tag_prefix_uses_package_name_when_not_root() {
        let mut pkg = create_test_package("api");
        pkg.path = "packages/api".to_string();
        assert_eq!(resolve_tag_prefix(&pkg), "api-v");
    }

    #[test]
    fn resolve_tag_prefix_uses_default_at_root() {
        let pkg = create_test_package("test");
        assert_eq!(resolve_tag_prefix(&pkg), "v");
    }

    #[test]
    fn resolve_auto_start_next_precedence() {
        let mut pkg = create_test_package("test");

        // Package > global > default
        pkg.auto_start_next = Some(true);
        assert!(resolve_auto_start_next(&pkg, Some(false)));

        // Global > default
        pkg.auto_start_next = None;
        assert!(resolve_auto_start_next(&pkg, Some(true)));

        // Default
        assert!(!resolve_auto_start_next(&pkg, None));
    }

    #[test]
    fn resolve_prerelease_returns_none_without_suffix() {
        let pkg = create_test_package("test");
        let global = PrereleaseConfig::default();
        let global_overrides = GlobalOverrides::default();
        let package_overrides = HashMap::new();

        let result = resolve_prerelease(
            &pkg,
            &global,
            &global_overrides,
            &package_overrides,
        )
        .unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn resolve_prerelease_with_global_suffix() {
        let pkg = create_test_package("test");
        let global = PrereleaseConfig {
            suffix: Some("beta".to_string()),
            strategy: PrereleaseStrategy::Versioned,
        };
        let global_overrides = GlobalOverrides::default();
        let package_overrides = HashMap::new();

        let result = resolve_prerelease(
            &pkg,
            &global,
            &global_overrides,
            &package_overrides,
        )
        .unwrap()
        .unwrap();

        assert_eq!(result.suffix, Some("beta".to_string()));
    }

    #[test]
    fn resolve_version_increment_flags_uses_package_config() {
        let mut pkg = create_test_package("test");
        pkg.breaking_always_increment_major = Some(false);
        pkg.features_always_increment_minor = Some(false);

        let (breaking, features) =
            resolve_version_increment_flags(&pkg, true, true);

        assert!(!breaking);
        assert!(!features);
    }

    #[test]
    fn resolve_version_increment_flags_uses_global_defaults() {
        let pkg = create_test_package("test");

        let (breaking, features) =
            resolve_version_increment_flags(&pkg, true, false);

        assert!(breaking);
        assert!(!features);
    }
}
