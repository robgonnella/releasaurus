use merge::Merge;

use crate::config::{
    changelog::{ChangelogConfig, DEFAULT_BODY, NAMED_PARSERS},
    package::PackageConfig,
};

pub fn resolve_changelog_config(
    package_config: &PackageConfig,
    resolved_global_changelog_config: &ChangelogConfig,
) -> ChangelogConfig {
    let mut package_changelog = package_config.changelog.clone();

    merge::option::recurse(
        &mut package_changelog,
        Some(resolved_global_changelog_config.clone()),
    );

    // get global parsers
    let mut parsers = resolved_global_changelog_config
        .named_parsers
        .clone()
        .unwrap_or_default();

    // get package changelog config
    let mut changelog_config = package_changelog.unwrap_or_default();

    // get package parsers
    let package_parsers = changelog_config.named_parsers.unwrap_or_default();

    // override global parsers with package specific parsers
    parsers.extend(package_parsers);

    // merge user defined parsers with default fields
    for (group, parser) in parsers.iter_mut() {
        // Every Group variant is present in NAMED_PARSERS, so the
        // lookup always succeeds.
        parser.merge(NAMED_PARSERS[group].clone());
        log::info!("updated parser: group={group}, parser={parser:?}");
    }

    // fill in package parsers with defaults where missing
    for (group, parser) in NAMED_PARSERS.iter() {
        if !parsers.contains_key(group) {
            parsers.insert(*group, parser.clone());
        }
    }

    changelog_config.named_parsers = Some(parsers);

    changelog_config.body = changelog_config.body.or(Some(DEFAULT_BODY.into()));

    changelog_config
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use crate::{
        config::changelog::{DEFAULT_BODY, Group, Parser},
        resolver::resolvers::test_helper::create_test_package,
    };

    use super::*;

    #[test]
    fn resolve_changelog_config_precedence() {
        let mut pkg = create_test_package("test");
        let pkg_body = Some("pkg body".to_string());
        let global_body = Some("global body".to_string());

        pkg.changelog = Some(ChangelogConfig {
            body: pkg_body.clone(),
            ..ChangelogConfig::default()
        });

        let global_config = ChangelogConfig {
            body: global_body.clone(),
            ..ChangelogConfig::default()
        };

        // Package (global empty)
        let config =
            resolve_changelog_config(&pkg, &ChangelogConfig::default());

        assert_eq!(config.body, pkg_body);

        // Package (global not empty)
        let config = resolve_changelog_config(&pkg, &global_config);

        assert_eq!(config.body, pkg_body);

        // Global (package empty)
        let config =
            resolve_changelog_config(&PackageConfig::default(), &global_config);

        assert_eq!(config.body, global_body);

        // Default (both empty)
        let config = resolve_changelog_config(
            &PackageConfig::default(),
            &ChangelogConfig::default(),
        );

        assert_eq!(config.body, Some(DEFAULT_BODY.into()));
    }

    #[test]
    fn resolve_changelog_config_merges_named_parsers() {
        // Global overrides: skip features and CI.
        let global = ChangelogConfig {
            named_parsers: Some(IndexMap::from([
                (
                    Group::Feature,
                    Parser {
                        pattern: None,
                        title: None,
                        skip: Some(true),
                    },
                ),
                (
                    Group::CI,
                    Parser {
                        pattern: None,
                        title: None,
                        skip: Some(true),
                    },
                ),
            ])),
            ..ChangelogConfig::default()
        };

        // Package overrides: un-skip features (conflicts with global) and
        // skip chore. Only `skip` is set, so pattern/title must fall back to
        // the built-in defaults.
        let mut pkg = create_test_package("test");
        pkg.changelog = Some(ChangelogConfig {
            named_parsers: Some(IndexMap::from([
                (
                    Group::Feature,
                    Parser {
                        pattern: None,
                        title: None,
                        skip: Some(false),
                    },
                ),
                (
                    Group::Chore,
                    Parser {
                        pattern: None,
                        title: None,
                        skip: Some(true),
                    },
                ),
            ])),
            ..ChangelogConfig::default()
        });

        let config = resolve_changelog_config(&pkg, &global);
        let parsers = config.named_parsers.unwrap();

        // Every built-in group is present after defaults are filled in.
        assert_eq!(parsers.len(), NAMED_PARSERS.len());
        // Package wins over global on a conflicting group.
        assert_eq!(parsers[&Group::Feature].skip, Some(false));
        // Global-only override is preserved (package never touched CI).
        assert_eq!(parsers[&Group::CI].skip, Some(true));
        // Package-only override is applied.
        assert_eq!(parsers[&Group::Chore].skip, Some(true));
        // An untouched group keeps the built-in default.
        assert_eq!(parsers[&Group::Fix].skip, Some(false));

        // A partial parser (only `skip`) inherits pattern/title from default.
        let feature = &parsers[&Group::Feature];
        assert_eq!(
            feature.pattern.as_ref().unwrap().as_str(),
            NAMED_PARSERS[&Group::Feature]
                .pattern
                .as_ref()
                .unwrap()
                .as_str()
        );
        assert_eq!(feature.title, NAMED_PARSERS[&Group::Feature].title);
    }

    #[test]
    fn resolve_changelog_config_inherits_scalars_from_global() {
        let global = ChangelogConfig {
            body: Some("global body".into()),
            include_author: Some(true),
            skip_merge_commits: Some(false),
            aggregate_prereleases: Some(true),
            ..ChangelogConfig::default()
        };

        // Partial package config sets only `aggregate_prereleases`; every
        // other scalar is omitted (None) and must inherit from global.
        let mut pkg = create_test_package("test");
        pkg.changelog = Some(ChangelogConfig {
            aggregate_prereleases: Some(false),
            ..ChangelogConfig::default()
        });

        let config = resolve_changelog_config(&pkg, &global);

        // Omitted scalars inherit the global value.
        assert_eq!(config.include_author, Some(true));
        assert_eq!(config.skip_merge_commits, Some(false));
        assert_eq!(config.body, Some("global body".to_string()));
        // The explicitly-set scalar wins.
        assert_eq!(config.aggregate_prereleases, Some(false));
    }

    #[test]
    fn resolve_changelog_config_appends_custom_parsers() {
        let global = ChangelogConfig {
            custom_parsers: vec![Parser {
                pattern: None,
                title: Some("global custom".into()),
                skip: Some(false),
            }],
            ..ChangelogConfig::default()
        };

        let mut pkg = create_test_package("test");
        pkg.changelog = Some(ChangelogConfig {
            custom_parsers: vec![Parser {
                pattern: None,
                title: Some("package custom".into()),
                skip: Some(false),
            }],
            ..ChangelogConfig::default()
        });

        let config = resolve_changelog_config(&pkg, &global);
        let titles: Vec<_> = config
            .custom_parsers
            .iter()
            .filter_map(|p| p.title.clone())
            .collect();

        assert_eq!(config.custom_parsers.len(), 2);
        assert!(titles.contains(&"global custom".to_string()));
        assert!(titles.contains(&"package custom".to_string()));
    }
}
