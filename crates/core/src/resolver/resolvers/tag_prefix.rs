use std::collections::HashMap;

use crate::config::{
    package::{DEFAULT_TAG_PREFIX, PackageConfig},
    resolved::{GlobalOverrides, PackageOverrides},
};

/// Resolves the tag prefix for a package.
///
/// Logic:
/// - If explicitly set in config, use that
/// - If package is not at root, use `{package_name}-v`
/// - Otherwise use default `v`
pub fn resolve_tag_prefix(
    resolved_name: &str,
    package: &PackageConfig,
    package_overrides: &HashMap<String, PackageOverrides>,
    global_overrides: &GlobalOverrides,
) -> String {
    if let Some(overrides) = package_overrides.get(resolved_name)
        && let Some(prefix) = overrides.tag_prefix.as_ref()
    {
        return prefix.clone();
    }

    if let Some(prefix) = global_overrides.tag_prefix.as_ref() {
        return prefix.clone();
    }

    if let Some(prefix) = package.tag_prefix.as_ref() {
        return prefix.clone();
    }

    if package.workspace_root != "." || package.path != "." {
        format!("{}-v", resolved_name)
    } else {
        DEFAULT_TAG_PREFIX.to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::resolver::resolvers::test_helper::create_test_package;

    use super::*;

    #[test]
    fn resolve_tag_prefix_uses_explicit_config() {
        let mut pkg = create_test_package("test");
        pkg.tag_prefix = Some("custom-v".to_string());
        let resolved_name = pkg.name.clone();
        let package_overrides = HashMap::new();
        let global_overrides = GlobalOverrides::default();
        assert_eq!(
            resolve_tag_prefix(
                &resolved_name,
                &pkg,
                &package_overrides,
                &global_overrides
            ),
            "custom-v"
        );
    }

    #[test]
    fn resolve_tag_prefix_uses_package_name_when_not_root() {
        let mut pkg = create_test_package("api");
        pkg.path = "packages/api".to_string();
        let resolved_name = pkg.name.clone();
        let package_overrides = HashMap::new();
        let global_overrides = GlobalOverrides::default();
        assert_eq!(
            resolve_tag_prefix(
                &resolved_name,
                &pkg,
                &package_overrides,
                &global_overrides
            ),
            "api-v"
        );
    }

    #[test]
    fn resolve_tag_prefix_uses_default_at_root() {
        let pkg = create_test_package("test");
        let resolved_name = pkg.name.clone();
        let package_overrides = HashMap::new();
        let global_overrides = GlobalOverrides::default();
        assert_eq!(
            resolve_tag_prefix(
                &resolved_name,
                &pkg,
                &package_overrides,
                &global_overrides
            ),
            "v"
        );
    }

    #[test]
    fn resolve_tag_prefix_package_override_takes_precedence() {
        let mut pkg = create_test_package("my-pkg");
        pkg.tag_prefix = Some("config-v".to_string());

        let mut package_overrides = HashMap::new();
        package_overrides.insert(
            "my-pkg".to_string(),
            PackageOverrides {
                tag_prefix: Some("cli-v".to_string()),
                prerelease_suffix: None,
                prerelease_strategy: None,
            },
        );

        let global_overrides = GlobalOverrides {
            tag_prefix: Some("global-v".to_string()),
            ..GlobalOverrides::default()
        };

        assert_eq!(
            resolve_tag_prefix(
                "my-pkg",
                &pkg,
                &package_overrides,
                &global_overrides
            ),
            "cli-v"
        );
    }

    #[test]
    fn resolve_tag_prefix_global_override_over_config() {
        let mut pkg = create_test_package("my-pkg");
        pkg.tag_prefix = Some("config-v".to_string());

        let package_overrides = HashMap::new();
        let global_overrides = GlobalOverrides {
            tag_prefix: Some("global-v".to_string()),
            ..GlobalOverrides::default()
        };

        assert_eq!(
            resolve_tag_prefix(
                "my-pkg",
                &pkg,
                &package_overrides,
                &global_overrides
            ),
            "global-v"
        );
    }
}
