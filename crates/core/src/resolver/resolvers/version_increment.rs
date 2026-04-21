use crate::config::package::PackageConfig;

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
    use crate::resolver::resolvers::test_helper::create_test_package;

    use super::*;

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
