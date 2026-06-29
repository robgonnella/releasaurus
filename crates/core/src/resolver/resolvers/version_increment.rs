use std::collections::HashMap;

use crate::config::{
    VersionType,
    package::PackageConfig,
    resolved::{GlobalOverrides, PackageOverrides},
};

/// Resolves version_type configuration with override logic.
///
/// Precedence (highest to lowest):
/// 1. Package-level CLI overrides
/// 2. Global CLI overrides
/// 3. Package-level config
/// 4. Global config
///
/// Returns Default if no version_type is set after all resolution.
pub fn resolve_version_type(
    resolved_name: &str,
    package: &PackageConfig,
    global_version_type: Option<VersionType>,
    global_overrides: &GlobalOverrides,
    package_overrides: &HashMap<String, PackageOverrides>,
) -> VersionType {
    let mut version_type = global_version_type.unwrap_or_default();

    // Package config overrides global config
    if let Some(v_type) = package.version_type {
        version_type = v_type;
    }

    // Global CLI overrides override config
    if let Some(v_type) = global_overrides.version_type {
        version_type = v_type;
    }

    // Package-level CLI overrides override everything
    if let Some(overrides) = package_overrides.get(resolved_name)
        && let Some(v_type) = overrides.version_type
    {
        version_type = v_type;
    }

    version_type
}

/// Resolves version increment flags (breaking/features).
///
/// Precedence: package config > global config. Returns `None` when neither
/// is set; the default is applied later, only when a semantic version
/// updater is actually built.
pub fn resolve_version_increment_flags(
    package: &PackageConfig,
    global_breaking: Option<bool>,
    global_features: Option<bool>,
) -> (Option<bool>, Option<bool>) {
    let breaking = package.breaking_always_increment_major.or(global_breaking);
    let features = package.features_always_increment_minor.or(global_features);
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
    use crate::resolver::resolvers::test_helper::create_test_package;

    use super::*;

    #[test]
    fn resolve_version_increment_flags_uses_package_config() {
        let mut pkg = create_test_package("test");
        pkg.breaking_always_increment_major = Some(false);
        pkg.features_always_increment_minor = Some(false);

        let (breaking, features) =
            resolve_version_increment_flags(&pkg, Some(true), Some(true));

        assert_eq!(breaking, Some(false));
        assert_eq!(features, Some(false));
    }

    #[test]
    fn resolve_version_increment_flags_uses_global_config() {
        let pkg = create_test_package("test");

        let (breaking, features) =
            resolve_version_increment_flags(&pkg, Some(true), Some(false));

        assert_eq!(breaking, Some(true));
        assert_eq!(features, Some(false));
    }

    #[test]
    fn resolve_version_increment_flags_returns_none_when_unset() {
        let pkg = create_test_package("test");

        let (breaking, features) =
            resolve_version_increment_flags(&pkg, None, None);

        assert_eq!(breaking, None);
        assert_eq!(features, None);
    }
}
