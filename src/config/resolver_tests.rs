use std::collections::HashMap;

use crate::{
    cli::{CommitModifiers, GlobalOverrides, PackageOverrides},
    config::{
        Config,
        changelog::{ChangelogConfig, RewordedCommit},
        package::{AdditionalManifest, AdditionalManifestSpec, PackageConfig},
        prerelease::{PrereleaseConfig, PrereleaseStrategy},
        resolver::ConfigResolverBuilder,
    },
    error::ReleasaurusError,
    updater::generic::updater::GENERIC_VERSION_REGEX_PATTERN,
};

#[test]
fn resolve_sets_base_branch_from_global_override() {
    let config = Config::default();
    let global_overrides = GlobalOverrides {
        base_branch: Some("develop".into()),
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(global_overrides)
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    assert_eq!(resolved.base_branch, Some("develop".into()));
}

#[test]
fn resolve_sets_base_branch_from_config() {
    let config = Config {
        base_branch: Some("staging".into()),
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    assert_eq!(resolved.base_branch, Some("staging".into()));
}

#[test]
fn resolve_uses_repo_default_branch_as_fallback() {
    let config = Config::default();

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("trunk")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    assert_eq!(resolved.base_branch, Some("trunk".into()));
}

#[test]
fn resolve_derives_package_name_from_repo() {
    let config = Config {
        packages: vec![PackageConfig {
            path: ".".into(),
            workspace_root: ".".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    assert_eq!(resolved.base_branch, Some("main".into()));
    assert_eq!(resolved.packages[0].name, "my-repo");
}

#[test]
fn resolve_derives_package_name_from_path() {
    let config = Config {
        packages: vec![PackageConfig {
            path: "packages/api".into(),
            workspace_root: ".".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    assert_eq!(resolved.packages[0].name, "api");
}

#[test]
fn resolve_preserves_explicit_package_name() {
    let config = Config {
        packages: vec![PackageConfig {
            name: "custom-name".into(),
            path: "packages/api".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    assert_eq!(resolved.packages[0].name, "custom-name");
}

#[test]
fn resolve_sets_default_tag_prefix_for_root() {
    let config = Config {
        packages: vec![PackageConfig {
            path: ".".into(),
            workspace_root: ".".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    assert_eq!(resolved.packages[0].tag_prefix, Some("v".into()));
}

#[test]
fn resolve_sets_package_name_tag_prefix_for_subdir() {
    let config = Config {
        packages: vec![PackageConfig {
            path: "packages/api".into(),
            workspace_root: ".".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    assert_eq!(resolved.packages[0].tag_prefix, Some("api-v".into()));
}

#[test]
fn resolve_sets_configured_package_tag_prefix() {
    let config = Config {
        packages: vec![PackageConfig {
            path: "packages/api".into(),
            workspace_root: ".".into(),
            tag_prefix: Some("my-prefix-v".into()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    assert_eq!(resolved.packages[0].tag_prefix, Some("my-prefix-v".into()));
}

#[test]
fn resolve_prerelease_package_overrides_global() {
    let config = Config {
        prerelease: PrereleaseConfig {
            suffix: Some("alpha".into()),
            strategy: PrereleaseStrategy::Static,
        },
        packages: vec![PackageConfig {
            name: "pkg".into(),
            prerelease: Some(PrereleaseConfig {
                suffix: Some("beta".into()),
                strategy: PrereleaseStrategy::Versioned,
            }),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    let prerelease = resolved.packages[0].prerelease.as_ref().unwrap();
    assert_eq!(prerelease.suffix, Some("beta".into()));
    assert_eq!(prerelease.strategy, PrereleaseStrategy::Versioned);
}

#[test]
fn resolve_prerelease_global_cli_overrides_config() {
    let config = Config {
        prerelease: PrereleaseConfig {
            suffix: Some("alpha".into()),
            strategy: PrereleaseStrategy::Static,
        },
        packages: vec![PackageConfig {
            name: "pkg".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let global_overrides = GlobalOverrides {
        prerelease_suffix: Some("rc".into()),
        prerelease_strategy: Some(PrereleaseStrategy::Versioned),
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(global_overrides)
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    let prerelease = resolved.packages[0].prerelease.as_ref().unwrap();
    assert_eq!(prerelease.suffix, Some("rc".into()));
    assert_eq!(prerelease.strategy, PrereleaseStrategy::Versioned);
}

#[test]
fn resolve_prerelease_package_cli_overrides_all() {
    let config = Config {
        prerelease: PrereleaseConfig {
            suffix: Some("alpha".into()),
            ..Default::default()
        },
        packages: vec![PackageConfig {
            name: "pkg".into(),
            prerelease: Some(PrereleaseConfig {
                suffix: Some("beta".into()),
                ..Default::default()
            }),
            ..Default::default()
        }],
        ..Default::default()
    };

    let global_overrides = GlobalOverrides {
        prerelease_suffix: Some("rc".into()),
        ..Default::default()
    };

    let mut package_overrides = HashMap::new();
    package_overrides.insert(
        "pkg".into(),
        PackageOverrides {
            prerelease_suffix: Some("gamma".into()),
            prerelease_strategy: Some(PrereleaseStrategy::Static),
        },
    );

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(package_overrides)
        .global_overrides(global_overrides)
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    let prerelease = resolved.packages[0].prerelease.as_ref().unwrap();
    assert_eq!(prerelease.suffix, Some("gamma".into()));
    assert_eq!(prerelease.strategy, PrereleaseStrategy::Static);
}

#[test]
fn resolve_removes_prerelease_when_suffix_empty() {
    let config = Config {
        packages: vec![PackageConfig {
            name: "pkg".into(),
            prerelease: Some(PrereleaseConfig {
                suffix: Some("".into()),
                ..Default::default()
            }),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    assert!(resolved.packages[0].prerelease.is_none());
}

#[test]
fn resolve_trims_prerelease_suffix() {
    let config = Config {
        packages: vec![PackageConfig {
            name: "pkg".into(),
            prerelease: Some(PrereleaseConfig {
                suffix: Some("  beta  ".into()),
                ..Default::default()
            }),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    let prerelease = resolved.packages[0].prerelease.as_ref().unwrap();
    assert_eq!(prerelease.suffix, Some("beta".into()));
}

#[test]
fn resolve_sets_analyzer_config_with_custom_regex() {
    let config = Config {
        custom_major_increment_regex: Some("BREAKING".into()),
        custom_minor_increment_regex: Some("FEATURE".into()),
        packages: vec![PackageConfig {
            name: "pkg".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    let analyzer_config = &resolved.packages[0].analyzer_config;
    assert_eq!(
        analyzer_config.custom_major_increment_regex,
        Some("BREAKING".into())
    );
    assert_eq!(
        analyzer_config.custom_minor_increment_regex,
        Some("FEATURE".into())
    );
}

#[test]
fn resolve_package_custom_regex_overrides_global() {
    let config = Config {
        custom_major_increment_regex: Some("GLOBAL".into()),
        packages: vec![PackageConfig {
            name: "pkg".into(),
            custom_major_increment_regex: Some("PACKAGE".into()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    let analyzer_config = &resolved.packages[0].analyzer_config;
    assert_eq!(
        analyzer_config.custom_major_increment_regex,
        Some("PACKAGE".into())
    );
}

#[test]
fn resolve_sets_analyzer_config_flags() {
    let config = Config {
        breaking_always_increment_major: false,
        features_always_increment_minor: false,
        packages: vec![PackageConfig {
            name: "pkg".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    let analyzer_config = &resolved.packages[0].analyzer_config;
    assert!(!analyzer_config.breaking_always_increment_major);
    assert!(!analyzer_config.features_always_increment_minor);
}

#[test]
fn resolve_shares_global_regex_across_multiple_packages() {
    let config = Config {
        custom_major_increment_regex: Some("^BREAKING:".into()),
        custom_minor_increment_regex: Some("^FEATURE:".into()),
        packages: vec![
            PackageConfig {
                name: "pkg1".into(),
                path: "packages/pkg1".into(),
                ..Default::default()
            },
            PackageConfig {
                name: "pkg2".into(),
                path: "packages/pkg2".into(),
                ..Default::default()
            },
            PackageConfig {
                name: "pkg3".into(),
                path: "packages/pkg3".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("my-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    // All packages should get the global regex config
    for package in resolved.packages.iter() {
        assert_eq!(
            package.analyzer_config.custom_major_increment_regex,
            Some("^BREAKING:".into()),
            "Package {} should have global major regex",
            package.name
        );
        assert_eq!(
            package.analyzer_config.custom_minor_increment_regex,
            Some("^FEATURE:".into()),
            "Package {} should have global minor regex",
            package.name
        );
    }
}

#[test]
fn resolve_rejects_invalid_skip_sha() {
    let config = Config {
        changelog: ChangelogConfig {
            skip_shas: Some(vec!["abc".into()]), // Too short
            ..Default::default()
        },
        packages: vec![PackageConfig {
            name: "pkg".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let result = resolver.resolve();

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid SHA in changelog.skip_shas")
    );
}

#[test]
fn resolve_rejects_invalid_reword_sha() {
    let config = Config {
        changelog: ChangelogConfig {
            reword: Some(vec![RewordedCommit {
                sha: "xyz".into(), // Too short
                message: "new message".into(),
            }]),
            ..Default::default()
        },
        packages: vec![PackageConfig {
            name: "pkg".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let result = resolver.resolve();

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid SHA in changelog.reword")
    );
}

#[test]
fn resolve_accepts_valid_skip_shas() {
    let config = Config {
        changelog: ChangelogConfig {
            skip_shas: Some(vec!["abc123d".into(), "def456e".into()]),
            ..Default::default()
        },
        packages: vec![PackageConfig {
            name: "pkg".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let result = resolver.resolve();

    assert!(result.is_ok());
}

#[test]
fn resolve_accepts_valid_reword_shas() {
    let config = Config {
        changelog: ChangelogConfig {
            reword: Some(vec![
                RewordedCommit {
                    sha: "abc123d".into(),
                    message: "fix: corrected message".into(),
                },
                RewordedCommit {
                    sha: "def456e".into(),
                    message: "feat: new feature".into(),
                },
            ]),
            ..Default::default()
        },
        packages: vec![PackageConfig {
            name: "pkg".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let result = resolver.resolve();

    assert!(result.is_ok());
}

#[test]
fn resolve_compiles_additional_manifest_regex_patterns() {
    let config = Config {
        packages: vec![PackageConfig {
            name: "test-pkg".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            additional_manifest_files: Some(vec![
                AdditionalManifestSpec::Path("VERSION".to_string()),
                AdditionalManifestSpec::Full(AdditionalManifest {
                    path: "config.yml".to_string(),
                    version_regex: Some(
                        r"version:\s*(?<version>\d+\.\d+\.\d+)".to_string(),
                    ),
                }),
            ]),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    let pkg = &resolved.packages[0];
    assert_eq!(pkg.compiled_additional_manifests.len(), 2);
    assert_eq!(pkg.compiled_additional_manifests[0].path, "VERSION");
    assert_eq!(pkg.compiled_additional_manifests[1].path, "config.yml");

    // Verify regexes are actually compiled and functional
    assert!(
        pkg.compiled_additional_manifests[0]
            .version_regex
            .is_match(r#"version = "1.0.0""#)
    );
    assert!(
        pkg.compiled_additional_manifests[1]
            .version_regex
            .is_match("version: 1.2.3")
    );
}

#[test]
fn resolve_rejects_invalid_regex_in_additional_manifests() {
    let config = Config {
        packages: vec![PackageConfig {
            name: "test-pkg".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            additional_manifest_files: Some(vec![
                AdditionalManifestSpec::Full(AdditionalManifest {
                    path: "VERSION".to_string(),
                    version_regex: Some(r"[invalid(regex".to_string()),
                }),
            ]),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let result = resolver.resolve();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));
}

#[test]
fn resolve_rejects_regex_without_version_capture_group() {
    let config = Config {
        packages: vec![PackageConfig {
            name: "test-pkg".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            additional_manifest_files: Some(vec![
                AdditionalManifestSpec::Full(AdditionalManifest {
                    path: "VERSION".to_string(),
                    version_regex: Some(
                        r"version:\s*(\d+\.\d+\.\d+)".to_string(),
                    ),
                }),
            ]),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let result = resolver.resolve();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("must include a named capture group"),
        "Expected error about missing version capture group, got: {}",
        err
    );
}

#[test]
fn resolve_uses_default_regex_when_none_specified() {
    let config = Config {
        packages: vec![PackageConfig {
            name: "test-pkg".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            additional_manifest_files: Some(vec![
                AdditionalManifestSpec::Path("VERSION".to_string()),
            ]),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    let pkg = &resolved.packages[0];
    assert_eq!(pkg.compiled_additional_manifests.len(), 1);

    // Verify the exact GENERIC_VERSION_REGEX_PATTERN is used when no custom pattern provided
    let compiled_regex = &pkg.compiled_additional_manifests[0].version_regex;
    assert_eq!(
        compiled_regex.as_str(),
        GENERIC_VERSION_REGEX_PATTERN,
        "Expected compiled regex to be GENERIC_VERSION_REGEX_PATTERN when version_regex is None"
    );

    // Also verify it works functionally
    assert!(compiled_regex.is_match(r#"version = "1.0.0""#));
    assert!(compiled_regex.is_match(r#"version: "2.3.4""#));
    assert!(compiled_regex.is_match(r#"VERSION='3.0.0'"#));
}

#[test]
fn resolve_empty_additional_manifests_produces_empty_compiled() {
    let config = Config {
        packages: vec![PackageConfig {
            name: "test-pkg".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            additional_manifest_files: None,
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    let pkg = &resolved.packages[0];
    assert_eq!(pkg.compiled_additional_manifests.len(), 0);
}

#[test]
fn resolve_converts_string_paths_to_manifests_with_default_regex() {
    let config = Config {
        packages: vec![PackageConfig {
            name: "test-pkg".into(),
            path: ".".into(),
            workspace_root: ".".into(),
            additional_manifest_files: Some(vec![
                AdditionalManifestSpec::Path("VERSION".to_string()),
                AdditionalManifestSpec::Path("README.md".to_string()),
            ]),
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolver = ConfigResolverBuilder::default()
        .config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url("https://example.com")
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let resolved = resolver.resolve().unwrap();

    let pkg = &resolved.packages[0];
    assert_eq!(pkg.compiled_additional_manifests.len(), 2);
    assert_eq!(pkg.compiled_additional_manifests[0].path, "VERSION");
    assert_eq!(pkg.compiled_additional_manifests[1].path, "README.md");

    // Both should use GENERIC_VERSION_REGEX_PATTERN
    assert_eq!(
        pkg.compiled_additional_manifests[0].version_regex.as_str(),
        GENERIC_VERSION_REGEX_PATTERN
    );
    assert_eq!(
        pkg.compiled_additional_manifests[1].version_regex.as_str(),
        GENERIC_VERSION_REGEX_PATTERN
    );
}
