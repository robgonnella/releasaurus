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

    /// `merge::option::recurse` merges the two tables field by field, so a
    /// package setting only `body` still inherits every other default.
    #[test]
    fn resolve_changelog_config_merges_field_by_field() {
        let mut pkg = create_test_package("test");

        pkg.changelog = Some(ChangelogConfig {
            body: Some("pkg body".into()),
            ..ChangelogConfig::default()
        });

        let defaults = ChangelogConfig {
            include_author: Some(true),
            include_pr_link: Some(true),
            aggregate_prereleases: Some(true),
            ..ChangelogConfig::default()
        };

        let config = resolve_changelog_config(&pkg, &defaults);

        assert_eq!(config.body, Some("pkg body".to_string()));
        assert_eq!(
            (
                config.include_author,
                config.include_pr_link,
                config.aggregate_prereleases
            ),
            (Some(true), Some(true), Some(true)),
            "fields the package left unset must survive the merge"
        );
    }

    /// An explicit `false` on the package is not the same as unset: the
    /// `overwrite_none` strategy only fills fields the package left `None`,
    /// so an opt-out survives a `[defaults]` that opts in — while sibling
    /// fields still inherit.
    #[test]
    fn resolve_changelog_config_keeps_an_explicit_false_over_defaults() {
        let mut pkg = create_test_package("test");

        pkg.changelog = Some(ChangelogConfig {
            include_pr_link: Some(false),
            ..ChangelogConfig::default()
        });

        let defaults = ChangelogConfig {
            include_pr_link: Some(true),
            include_author: Some(true),
            ..ChangelogConfig::default()
        };

        let config = resolve_changelog_config(&pkg, &defaults);

        assert_eq!(
            config.include_pr_link,
            Some(false),
            "defaults must not overwrite an explicit opt-out"
        );
        assert_eq!(
            config.include_author,
            Some(true),
            "an opt-out on one field must not block inheriting others"
        );
    }
}
