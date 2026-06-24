use crate::config::{
    changelog::{ChangelogConfig, DEFAULT_BODY},
    package::PackageConfig,
};

pub fn resolve_changelog_config(
    package_config: &PackageConfig,
    resolved_default_changelog_config: &ChangelogConfig,
) -> ChangelogConfig {
    let mut package_changelog = package_config.changelog.clone();

    merge::option::recurse(
        &mut package_changelog,
        Some(resolved_default_changelog_config.clone()),
    );

    // get package changelog config
    let mut changelog_config = package_changelog.unwrap_or_default();

    changelog_config.body = changelog_config.body.or(Some(DEFAULT_BODY.into()));

    changelog_config
}

#[cfg(test)]
mod tests {
    use crate::{
        config::changelog::DEFAULT_BODY,
        resolver::resolvers::test_helper::create_test_package,
    };

    use super::*;

    #[test]
    fn resolve_changelog_config_precedence() {
        let mut pkg = create_test_package("test");
        let pkg_body = Some("pkg body".to_string());
        let default_body = Some("default body".to_string());

        pkg.changelog = Some(ChangelogConfig {
            body: pkg_body.clone(),
            ..ChangelogConfig::default()
        });

        let default_config = ChangelogConfig {
            body: default_body.clone(),
            ..ChangelogConfig::default()
        };

        // Package (default empty)
        let config =
            resolve_changelog_config(&pkg, &ChangelogConfig::default());

        assert_eq!(config.body, pkg_body);

        // Package (default not empty)
        let config = resolve_changelog_config(&pkg, &default_config);

        assert_eq!(config.body, pkg_body);

        // Default (package empty)
        let config = resolve_changelog_config(
            &PackageConfig::default(),
            &default_config,
        );

        assert_eq!(config.body, default_body);

        // Default (both empty)
        let config = resolve_changelog_config(
            &PackageConfig::default(),
            &ChangelogConfig::default(),
        );

        assert_eq!(config.body, Some(DEFAULT_BODY.into()));
    }
}
