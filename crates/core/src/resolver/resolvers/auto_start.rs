use crate::config::package::PackageConfig;

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

#[cfg(test)]
mod tests {
    use crate::resolver::resolvers::test_helper::create_test_package;

    use super::*;

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
}
