use crate::config::{
    overrides::{GlobalOverrides, PackageOverridesHash},
    package::PackageConfig,
    prerelease::PrereleaseConfig,
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
///
/// `name` is the *resolved* package name from [`resolve_package_name`], not
/// `package.name`: the latter is empty when the TOML omits `name`, while
/// per-package overrides are addressed by the name the user sees.
///
/// [`resolve_package_name`]: super::package_name::resolve_package_name
pub fn resolve_prerelease(
    name: &str,
    package: &PackageConfig,
    global_prerelease: &PrereleaseConfig,
    global_overrides: &GlobalOverrides,
    package_overrides: &PackageOverridesHash,
) -> Option<PrereleaseConfig> {
    let mut prerelease = global_prerelease.clone();

    // Package config overrides global config
    if let Some(version_config) = package.versioning.as_ref()
        && let Some(pkg_prerelease) = version_config.prerelease.clone()
    {
        prerelease = pkg_prerelease;
    }

    // Global CLI overrides override config
    if let Some(ref suffix) = global_overrides.prerelease_suffix {
        prerelease.suffix = suffix.clone();
    }
    if let Some(strategy) = global_overrides.prerelease_strategy {
        prerelease.strategy = strategy;
    }

    // Package-level CLI overrides override everything
    if let Some(overrides) = package_overrides.get(name) {
        if let Some(ref suffix) = overrides.prerelease_suffix {
            prerelease.suffix = suffix.clone();
        }
        if let Some(strategy) = overrides.prerelease_strategy {
            prerelease.strategy = strategy;
        }
    }

    // Clean and validate suffix
    prerelease.suffix = prerelease.suffix.trim().to_string();

    if prerelease.suffix.is_empty() {
        None
    } else {
        Some(prerelease)
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
        let package_overrides = PackageOverridesHash::new();

        let result = resolve_prerelease(
            "test",
            &pkg,
            &global,
            &global_overrides,
            &package_overrides,
        );

        assert!(result.is_none());
    }

    #[test]
    fn resolve_prerelease_with_default_suffix() {
        let pkg = create_test_package("test");
        let default = PrereleaseConfig {
            suffix: "beta".to_string(),
            strategy: PrereleaseStrategy::Versioned,
        };
        let global_overrides = GlobalOverrides::default();
        let package_overrides = PackageOverridesHash::new();

        let result = resolve_prerelease(
            "test",
            &pkg,
            &default,
            &global_overrides,
            &package_overrides,
        )
        .unwrap();

        assert_eq!(result.suffix, "beta".to_string());
    }

    /// A package's `prerelease` table replaces the default one rather than
    /// merging into it: `suffix` and `strategy` describe one prerelease
    /// identity, so a package naming a new suffix gets the default strategy,
    /// not the global one. This is deliberate - see "Per-package overrides"
    /// in the configuration reference.
    ///
    /// Built from TOML rather than a struct literal so the omitted `strategy`
    /// is filled in by serde, exactly as it would be for a real config.
    #[test]
    fn resolve_prerelease_package_table_replaces_default() {
        let pkg: PackageConfig = toml::from_str(
            r#"
            name = "test"
            versioning = { prerelease = { suffix = "alpha" } }
        "#,
        )
        .unwrap();

        let default = PrereleaseConfig {
            suffix: "SNAPSHOT".to_string(),
            strategy: PrereleaseStrategy::Static,
        };

        let result = resolve_prerelease(
            "test",
            &pkg,
            &default,
            &GlobalOverrides::default(),
            &PackageOverridesHash::new(),
        )
        .unwrap();

        assert_eq!(result.suffix, "alpha".to_string());
        assert_eq!(result.strategy, PrereleaseStrategy::Versioned);
    }
}
