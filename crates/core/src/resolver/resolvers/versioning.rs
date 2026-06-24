use indexmap::IndexMap;
use merge::Merge;

use crate::{
    config::{
        package::PackageConfig,
        resolved::ResolvedConfig,
        versioning::{
            Group, MAX_PARSER_ORDER, NAMED_PARSERS, Parser, VersioningConfig,
        },
    },
    resolver::resolvers::prerelease::resolve_prerelease,
    result::{ReleasaurusError, Result},
};

/// Resolves all versioning config with package config taking precedence
pub fn resolve_versioning(
    resolved_config: &ResolvedConfig,
    package_config: &PackageConfig,
) -> Result<VersioningConfig> {
    let default_versioning = resolved_config.versioning.as_ref();
    let package_versioning = package_config.versioning.as_ref();

    // Package config is the left side of the merge. Every scalar field uses
    // the `overwrite_none` strategy, so package values win and global values
    // only fill in the fields the package left unset. The two parser fields
    // are excluded from this merge and resolved separately below.
    let mut final_versioning = package_versioning.cloned();

    merge::option::recurse(&mut final_versioning, default_versioning.cloned());

    let mut final_versioning = final_versioning.unwrap_or_default();

    // Prerelease has its own precedence chain (config, then global CLI
    // overrides, then per-package CLI overrides), so it is resolved
    // separately and assigned over whatever the merge above produced.
    let final_prerelease_config = resolve_prerelease(
        package_config,
        &default_versioning
            .and_then(|v| v.prerelease.clone())
            .unwrap_or_default(),
        &resolved_config.global_overrides,
        &resolved_config.package_overrides,
    );

    // Parsers merge per group and per field rather than whole-value, so they
    // are also resolved separately.
    let named_parsers = resolve_named_parsers(
        default_versioning.and_then(|v| v.named_parsers.as_ref()),
        package_versioning.and_then(|v| v.named_parsers.as_ref()),
    );

    validate_named_parsers(&named_parsers)?;

    final_versioning.named_parsers = Some(named_parsers);

    validate_custom_parsers(&final_versioning)?;

    final_versioning.prerelease = final_prerelease_config;

    Ok(final_versioning)
}

/// Layers user overrides on top of [`NAMED_PARSERS`] field by field.
///
/// Precedence per field is package, then global, then the built-in
/// default. Merging per field (rather than replacing whole parsers) means
/// a package that only re-titles a group still inherits the global `skip`
/// and `pattern` for it.
///
/// The result always contains every [`Group`], in [`NAMED_PARSERS`] order,
/// so match order does not depend on the order keys appear in the user's
/// TOML.
fn resolve_named_parsers(
    default_parsers: Option<&IndexMap<Group, Parser>>,
    package_parsers: Option<&IndexMap<Group, Parser>>,
) -> IndexMap<Group, Parser> {
    let mut parsers = IndexMap::with_capacity(NAMED_PARSERS.len());

    for (group, default_parser) in NAMED_PARSERS.iter() {
        let mut parser = package_parsers
            .and_then(|p| p.get(group))
            .cloned()
            .unwrap_or_default();

        if let Some(default_parser) = default_parsers.and_then(|p| p.get(group))
        {
            parser.merge(default_parser.clone());
        }

        parser.merge(default_parser.clone());

        log::debug!("resolved parser: group={group}, parser={parser:?}");

        parsers.insert(*group, parser);
    }

    parsers
}

/// Rejects named parsers whose resolved title is blank or carries a stale
/// order tag, and orders outside the two-digit range.
///
/// A blank title still matches commits, but groups them under an empty
/// changelog heading. `overwrite_none` only fills in `None`, so an explicit
/// `title = ""` survives the merge with the built-in default and has to be
/// caught here.
fn validate_named_parsers(parsers: &IndexMap<Group, Parser>) -> Result<()> {
    for (group, parser) in parsers.iter() {
        let title = parser.title.as_deref().map(str::trim);

        if title.is_none_or(str::is_empty) {
            return Err(ReleasaurusError::invalid_config(format!(
                r#"named parser "{group}" has a blank "title": matching commits would be grouped under an empty changelog heading"#
            )));
        }

        validate_title_has_no_order_tag(title.unwrap_or_default(), group)?;
        validate_order_in_range(parser.order, group)?;
    }

    Ok(())
}

/// Rejects a title carrying a literal `<!-- NN -->` tag.
///
/// The tag is synthesized from `order` (see [`Parser::group_title`]), so one
/// written by hand would be double-tagged and silently take precedence over
/// the `order` the user set.
fn validate_title_has_no_order_tag(
    title: &str,
    label: impl std::fmt::Display,
) -> Result<()> {
    if title.contains("<!--") {
        return Err(ReleasaurusError::invalid_config(format!(
            r#"parser "{label}" has an HTML comment in its "title": changelog order is set with the "order" field now, so the title should hold only the heading text"#
        )));
    }

    Ok(())
}

/// Rejects an `order` above [`MAX_PARSER_ORDER`].
///
/// Order becomes a fixed two-digit sort tag, so a wider value would sort
/// by its first digit instead of its magnitude.
fn validate_order_in_range(
    order: Option<u8>,
    label: impl std::fmt::Display,
) -> Result<()> {
    if let Some(order) = order
        && order > MAX_PARSER_ORDER
    {
        return Err(ReleasaurusError::invalid_config(format!(
            r#"parser "{label}" has "order" {order}: must be between 0 and {MAX_PARSER_ORDER}"#
        )));
    }

    Ok(())
}

/// Rejects custom parsers that would silently do nothing.
///
/// Unlike named parsers, custom parsers have no built-in defaults to fall
/// back on: one without a `pattern` never matches, one without a `title`
/// groups its commits under an empty changelog heading, and one without an
/// `order` has no defined position among the built-in groups.
fn validate_custom_parsers(versioning: &VersioningConfig) -> Result<()> {
    let Some(custom_parsers) = versioning.custom_parsers.as_ref() else {
        return Ok(());
    };

    for parser in custom_parsers.0.iter() {
        let title = parser.title.as_deref().map(str::trim);

        let Some(pattern) = parser.pattern.as_ref() else {
            return Err(ReleasaurusError::invalid_config(format!(
                r#"custom parser "{}" is missing a "pattern": a custom parser without a pattern never matches any commit"#,
                title.unwrap_or_default()
            )));
        };

        if title.is_none_or(str::is_empty) {
            return Err(ReleasaurusError::invalid_config(format!(
                r#"custom parser with pattern "{}" is missing a "title": matching commits would be grouped under an empty changelog heading"#,
                pattern.as_str()
            )));
        }

        validate_title_has_no_order_tag(
            title.unwrap_or_default(),
            pattern.as_str(),
        )?;

        if parser.order.is_none() {
            return Err(ReleasaurusError::invalid_config(format!(
                r#"custom parser with pattern "{}" is missing an "order": custom groups have no built-in position, so "order" (0-{MAX_PARSER_ORDER}) is required"#,
                pattern.as_str()
            )));
        }

        validate_order_in_range(parser.order, pattern.as_str())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use regex::Regex;
    use url::Url;

    use crate::config::{
        changelog::ChangelogConfig,
        repository::{DEFAULT_COMMIT_SEARCH_DEPTH, DEFAULT_TAG_SEARCH_DEPTH},
        resolved::{CommitModifiers, GlobalOverrides},
        versioning::ParserList,
    };

    use super::*;

    fn make_resolved_config(
        versioning: Option<VersioningConfig>,
    ) -> ResolvedConfig {
        ResolvedConfig {
            repo_name: "test-repo".into(),
            base_branch: "main".into(),
            release_link_base_url: Url::parse("https://example.com/").unwrap(),
            compare_link_base_url: Url::parse("https://example.com/compare/")
                .unwrap(),
            package_overrides: HashMap::default(),
            global_overrides: GlobalOverrides::default(),
            commit_modifiers: CommitModifiers::default(),
            first_release_search_depth: DEFAULT_COMMIT_SEARCH_DEPTH,
            tag_search_depth: DEFAULT_TAG_SEARCH_DEPTH,
            separate_pull_requests: true,
            changelog: ChangelogConfig::default(),
            versioning,
        }
    }

    fn package_with_versioning(versioning: VersioningConfig) -> PackageConfig {
        PackageConfig {
            versioning: Some(versioning),
            ..PackageConfig::default()
        }
    }

    #[test]
    fn resolve_versioning_precedence() {
        // Package (global empty)
        let resolved = resolve_versioning(
            &make_resolved_config(None),
            &package_with_versioning(VersioningConfig {
                skip_merge_commits: Some(false),
                ..VersioningConfig::default()
            }),
        )
        .unwrap();

        assert_eq!(resolved.skip_merge_commits, Some(false));

        // Package (global not empty) - package wins
        let global = VersioningConfig {
            skip_merge_commits: Some(true),
            features_always_increment_minor: Some(true),
            ..VersioningConfig::default()
        };

        let resolved = resolve_versioning(
            &make_resolved_config(Some(global.clone())),
            &package_with_versioning(VersioningConfig {
                skip_merge_commits: Some(false),
                ..VersioningConfig::default()
            }),
        )
        .unwrap();

        assert_eq!(resolved.skip_merge_commits, Some(false));
        // fields the package left unset still come from global
        assert_eq!(resolved.features_always_increment_minor, Some(true));

        // Global (package empty)
        let resolved = resolve_versioning(
            &make_resolved_config(Some(global)),
            &PackageConfig::default(),
        )
        .unwrap();

        assert_eq!(resolved.skip_merge_commits, Some(true));

        // Default (both empty)
        let resolved = resolve_versioning(
            &make_resolved_config(None),
            &PackageConfig::default(),
        )
        .unwrap();

        assert_eq!(resolved.skip_merge_commits, None);
    }

    #[test]
    fn resolve_versioning_package_increment_flags_win_over_global() {
        let global = VersioningConfig {
            breaking_always_increment_major: Some(false),
            custom_minor_increment_regex: Some("^global".into()),
            auto_start_next: Some(false),
            ..VersioningConfig::default()
        };

        let resolved = resolve_versioning(
            &make_resolved_config(Some(global)),
            &package_with_versioning(VersioningConfig {
                breaking_always_increment_major: Some(true),
                custom_minor_increment_regex: Some("^package".into()),
                auto_start_next: Some(true),
                ..VersioningConfig::default()
            }),
        )
        .unwrap();

        assert_eq!(resolved.breaking_always_increment_major, Some(true));
        assert_eq!(
            resolved.custom_minor_increment_regex,
            Some("^package".into())
        );
        assert_eq!(resolved.auto_start_next, Some(true));
    }

    #[test]
    fn resolve_versioning_fills_all_named_parsers_with_defaults() {
        let resolved = resolve_versioning(
            &make_resolved_config(None),
            &PackageConfig::default(),
        )
        .unwrap();

        let parsers = resolved.named_parsers.unwrap();

        assert_eq!(parsers.len(), NAMED_PARSERS.len());

        for (group, default_parser) in NAMED_PARSERS.iter() {
            let parser = parsers.get(group).unwrap();
            assert_eq!(parser.title, default_parser.title);
            assert_eq!(parser.skip, default_parser.skip);
        }
    }

    #[test]
    fn resolve_versioning_named_parsers_merge_field_by_field() {
        // Global skips CI, package only retitles it. The package must keep
        // the global skip rather than resetting it to the built-in default.
        let global = VersioningConfig {
            named_parsers: Some(IndexMap::from([(
                Group::CI,
                Parser {
                    pattern: None,
                    title: None,
                    skip: Some(true),
                    order: None,
                },
            )])),
            ..VersioningConfig::default()
        };

        let package = package_with_versioning(VersioningConfig {
            named_parsers: Some(IndexMap::from([(
                Group::CI,
                Parser {
                    pattern: None,
                    title: Some("Pipelines".into()),
                    skip: None,
                    order: None,
                },
            )])),
            ..VersioningConfig::default()
        });

        let resolved =
            resolve_versioning(&make_resolved_config(Some(global)), &package)
                .unwrap();

        let ci = resolved
            .named_parsers
            .unwrap()
            .get(&Group::CI)
            .unwrap()
            .clone();

        assert_eq!(ci.title, Some("Pipelines".into()));
        assert_eq!(ci.skip, Some(true));
        // pattern was never set by either, so it falls back to the default
        assert_eq!(
            ci.pattern.map(|p| p.as_str().to_string()),
            NAMED_PARSERS[&Group::CI]
                .pattern
                .as_ref()
                .map(|p| p.as_str().to_string())
        );
    }

    #[test]
    fn resolve_versioning_package_named_parser_overrides_global() {
        let global = VersioningConfig {
            named_parsers: Some(IndexMap::from([(
                Group::Chore,
                Parser {
                    pattern: None,
                    title: None,
                    skip: Some(true),
                    order: None,
                },
            )])),
            ..VersioningConfig::default()
        };

        let package = package_with_versioning(VersioningConfig {
            named_parsers: Some(IndexMap::from([(
                Group::Chore,
                Parser {
                    pattern: None,
                    title: None,
                    skip: Some(false),
                    order: None,
                },
            )])),
            ..VersioningConfig::default()
        });

        let resolved =
            resolve_versioning(&make_resolved_config(Some(global)), &package)
                .unwrap();

        let chore = resolved
            .named_parsers
            .unwrap()
            .get(&Group::Chore)
            .unwrap()
            .clone();

        assert_eq!(chore.skip, Some(false));
    }

    #[test]
    fn resolve_versioning_named_parsers_always_in_default_order() {
        // Only `chore` is customized, but the resolved map must still be in
        // NAMED_PARSERS order so match order is independent of TOML order.
        let global = VersioningConfig {
            named_parsers: Some(IndexMap::from([(
                Group::Chore,
                Parser {
                    pattern: None,
                    title: None,
                    skip: Some(true),
                    order: None,
                },
            )])),
            ..VersioningConfig::default()
        };

        let resolved = resolve_versioning(
            &make_resolved_config(Some(global)),
            &PackageConfig::default(),
        )
        .unwrap();

        let resolved_groups: Vec<Group> =
            resolved.named_parsers.unwrap().into_keys().collect();
        let default_groups: Vec<Group> =
            NAMED_PARSERS.keys().copied().collect();

        assert_eq!(resolved_groups, default_groups);
    }

    #[test]
    fn resolve_versioning_combines_global_and_package_custom_parsers() {
        let global = VersioningConfig {
            custom_parsers: Some(ParserList(vec![Parser::new(
                Some(Regex::new("^global").unwrap()),
                "Global".into(),
                false,
                0,
            )])),
            ..VersioningConfig::default()
        };

        let package = package_with_versioning(VersioningConfig {
            custom_parsers: Some(ParserList(vec![Parser::new(
                Some(Regex::new("^package").unwrap()),
                "Package".into(),
                false,
                0,
            )])),
            ..VersioningConfig::default()
        });

        let resolved =
            resolve_versioning(&make_resolved_config(Some(global)), &package)
                .unwrap();

        let titles: Vec<String> = resolved
            .custom_parsers
            .unwrap()
            .0
            .into_iter()
            .map(|p| p.title.unwrap())
            .collect();

        // package parsers are checked first
        assert_eq!(titles, vec!["Package".to_string(), "Global".to_string()]);
    }

    #[test]
    fn resolve_versioning_rejects_custom_parser_without_pattern() {
        let package = package_with_versioning(VersioningConfig {
            custom_parsers: Some(ParserList(vec![Parser {
                pattern: None,
                title: Some("Dependencies".into()),
                skip: Some(false),
                order: None,
            }])),
            ..VersioningConfig::default()
        });

        let result = resolve_versioning(&make_resolved_config(None), &package);

        let err = result.unwrap_err().to_string();
        assert!(err.contains("pattern"), "unexpected error: {err}");
        assert!(err.contains("Dependencies"), "unexpected error: {err}");
    }

    #[test]
    fn resolve_versioning_rejects_custom_parser_without_title() {
        let package = package_with_versioning(VersioningConfig {
            custom_parsers: Some(ParserList(vec![Parser {
                pattern: Some(Regex::new("^deps").unwrap()),
                title: Some("   ".into()),
                skip: Some(false),
                order: None,
            }])),
            ..VersioningConfig::default()
        });

        let result = resolve_versioning(&make_resolved_config(None), &package);

        let err = result.unwrap_err().to_string();
        assert!(err.contains("title"), "unexpected error: {err}");
        assert!(err.contains("^deps"), "unexpected error: {err}");
    }

    #[test]
    fn resolve_versioning_rejects_named_parser_with_blank_title() {
        // An explicit empty title survives the `overwrite_none` merge with the
        // built-in default, so it has to be rejected rather than rendering as
        // a bare "### " heading.
        let package = package_with_versioning(VersioningConfig {
            named_parsers: Some(IndexMap::from([(
                Group::CI,
                Parser {
                    pattern: None,
                    title: Some("   ".into()),
                    skip: None,
                    order: None,
                },
            )])),
            ..VersioningConfig::default()
        });

        let result = resolve_versioning(&make_resolved_config(None), &package);

        assert!(matches!(result, Err(ReleasaurusError::InvalidConfig(_))));
    }

    /// `order` merges field by field like the other parser fields, so a
    /// package can reposition a group without restating its title or skip.
    #[test]
    fn resolve_versioning_named_parser_order_merges_field_by_field() {
        let global = VersioningConfig {
            named_parsers: Some(IndexMap::from([(
                Group::Fix,
                Parser {
                    pattern: None,
                    title: Some("Fixes".into()),
                    skip: None,
                    order: Some(7),
                },
            )])),
            ..VersioningConfig::default()
        };

        // Package only moves the group; title comes from global, pattern and
        // skip from the built-in default.
        let package = package_with_versioning(VersioningConfig {
            named_parsers: Some(IndexMap::from([(
                Group::Fix,
                Parser {
                    pattern: None,
                    title: None,
                    skip: None,
                    order: Some(1),
                },
            )])),
            ..VersioningConfig::default()
        });

        let resolved =
            resolve_versioning(&make_resolved_config(Some(global)), &package)
                .unwrap();

        let fix = resolved.named_parsers.unwrap()[&Group::Fix].clone();

        assert_eq!(fix.order, Some(1));
        assert_eq!(fix.title, Some("Fixes".into()));
        assert_eq!(fix.group_title(), "<!-- 01 -->Fixes");
    }

    /// Groups the user never mentions keep their built-in order.
    #[test]
    fn resolve_versioning_named_parsers_inherit_default_order() {
        let resolved = resolve_versioning(
            &make_resolved_config(None),
            &PackageConfig::default(),
        )
        .unwrap();

        let parsers = resolved.named_parsers.unwrap();

        for (group, default_parser) in NAMED_PARSERS.iter() {
            assert_eq!(
                parsers[group].order, default_parser.order,
                "wrong order for {group}"
            );
        }
    }

    /// `order` has no built-in fallback for custom groups, so omitting it
    /// would leave the group's position undefined.
    #[test]
    fn resolve_versioning_rejects_custom_parser_without_order() {
        let package = package_with_versioning(VersioningConfig {
            custom_parsers: Some(ParserList(vec![Parser {
                pattern: Some(Regex::new("^deps").unwrap()),
                title: Some("Dependencies".into()),
                skip: Some(false),
                order: None,
            }])),
            ..VersioningConfig::default()
        });

        let result = resolve_versioning(&make_resolved_config(None), &package);

        assert!(matches!(result, Err(ReleasaurusError::InvalidConfig(_))));

        let err = result.unwrap_err().to_string();
        assert!(err.contains("order"), "unexpected error: {err}");
        assert!(err.contains("^deps"), "unexpected error: {err}");
    }

    /// Order becomes a two-digit sort tag, so a wider value would sort by
    /// its first digit rather than its magnitude.
    #[test]
    fn resolve_versioning_rejects_order_above_max() {
        let package = package_with_versioning(VersioningConfig {
            named_parsers: Some(IndexMap::from([(
                Group::CI,
                Parser {
                    pattern: None,
                    title: None,
                    skip: None,
                    order: Some(MAX_PARSER_ORDER + 1),
                },
            )])),
            ..VersioningConfig::default()
        });

        let result = resolve_versioning(&make_resolved_config(None), &package);

        assert!(matches!(result, Err(ReleasaurusError::InvalidConfig(_))));
    }

    /// The sort tag is synthesized from `order`, so one written by hand would
    /// double-tag the heading and quietly override the `order` that was set.
    #[test]
    fn resolve_versioning_rejects_title_carrying_an_order_tag() {
        let package = package_with_versioning(VersioningConfig {
            named_parsers: Some(IndexMap::from([(
                Group::Feature,
                Parser {
                    pattern: None,
                    title: Some("<!-- 01 -->🚀 Features".into()),
                    skip: None,
                    order: None,
                },
            )])),
            ..VersioningConfig::default()
        });

        let result = resolve_versioning(&make_resolved_config(None), &package);

        assert!(matches!(result, Err(ReleasaurusError::InvalidConfig(_))));
    }

    #[test]
    fn resolve_versioning_rejects_custom_parser_title_carrying_an_order_tag() {
        let package = package_with_versioning(VersioningConfig {
            custom_parsers: Some(ParserList(vec![Parser {
                pattern: Some(Regex::new("^deps").unwrap()),
                title: Some("<!-- 03 -->📦 Dependencies".into()),
                skip: Some(false),
                order: Some(3),
            }])),
            ..VersioningConfig::default()
        });

        let result = resolve_versioning(&make_resolved_config(None), &package);

        assert!(matches!(result, Err(ReleasaurusError::InvalidConfig(_))));
    }

    #[test]
    fn resolve_versioning_accepts_valid_custom_parser() {
        let package = package_with_versioning(VersioningConfig {
            custom_parsers: Some(ParserList(vec![Parser::new(
                Some(Regex::new("^deps").unwrap()),
                "📦 Dependencies".into(),
                false,
                3,
            )])),
            ..VersioningConfig::default()
        });

        let resolved =
            resolve_versioning(&make_resolved_config(None), &package).unwrap();

        assert_eq!(resolved.custom_parsers.unwrap().0.len(), 1);
    }
}
