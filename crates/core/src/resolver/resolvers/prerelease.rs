use std::collections::HashMap;

use crate::{
    config::{
        package::PackageConfig,
        prerelease::PrereleaseConfig,
        resolved::{GlobalOverrides, PackageOverrides},
    },
    result::Result,
};

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

#[cfg(test)]
mod tests {
    use crate::{
        config::prerelease::PrereleaseStrategy,
        resolver::resolvers::test_helper::create_test_package,
    };

    use super::*;

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
}
