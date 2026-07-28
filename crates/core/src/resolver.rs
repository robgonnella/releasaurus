use derive_builder::Builder;
use std::rc::Rc;
use url::Url;

use crate::{
    config::{
        Config,
        overrides::{CommitModifiers, GlobalOverrides, PackageOverridesHash},
        package::PackageConfig,
    },
    packages::resolved_hash::ResolvedPackageHash,
    resolver::resolvers::{
        base_branch::resolve_base_branch,
        commit_modifiers::resolve_commit_modifiers,
        package::{PackageResolverParams, resolve_package},
        templates::resolve_monorepo_templates,
    },
    result::{ReleasaurusError, Result},
};

pub mod resolvers;

/// Fully resolved runtime configuration for the release pipeline.
///
/// Produced by [`Resolver::resolve`] from the loaded TOML config, CLI
/// overrides, and forge metadata. Carries only what the pipeline needs
/// at runtime — everything else is consumed during resolution and
/// baked into the per-package
/// [`ResolvedPackage`][crate::packages::resolved::ResolvedPackage]s
/// held by `package_configs`.
pub struct ResolvedConfig {
    /// Repository name
    pub repo_name: String,
    /// Branch release PRs target and commits are read from.
    pub base_branch: String,
    /// Whether each package gets its own release PR.
    pub separate_pull_requests: bool,
    /// Template to use for commit messages when separate_pull_requests=false
    pub monorepo_commit_message_template: String,
    /// Template to use for PR titles when separate_pull_requests=false
    pub monorepo_pr_title_template: String,
    /// Resolved per-package config, indexed by package name.
    pub package_configs: ResolvedPackageHash,
    /// When `true`, an additional request is made per changelog commit to
    /// attach the PR that introduced it, if one exists. True when any package
    /// enables `include_pr_link`.
    pub pr_links_enabled: bool,
}

#[derive(Builder)]
#[builder(setter(into), build_fn(private, name = "_build"))]
pub struct Resolver {
    pub toml_config: Rc<Config>,
    pub repo_name: String,
    pub repo_default_branch: String,
    pub release_link_base_url: Url,
    pub compare_link_base_url: Url,
    pub package_overrides: PackageOverridesHash,
    pub global_overrides: GlobalOverrides,
    pub commit_modifiers: CommitModifiers,
}

impl ResolverBuilder {
    pub fn build(&self) -> Result<Resolver> {
        self._build().map_err(|e| {
            ReleasaurusError::invalid_config(format!(
                "Failed to build resolver: {}",
                e
            ))
        })
    }
}

impl Resolver {
    pub fn builder() -> ResolverBuilder {
        ResolverBuilder::default()
    }

    pub fn resolve(
        &self,
        packages: Vec<PackageConfig>,
    ) -> Result<Rc<ResolvedConfig>> {
        let base_branch = resolve_base_branch(
            &self.toml_config.repository,
            &self.global_overrides,
            &self.repo_default_branch,
        );

        let commit_modifiers = resolve_commit_modifiers(
            &self.toml_config.repository,
            &self.commit_modifiers,
        )?;

        let separate_pull_requests =
            self.toml_config.repository.separate_pull_requests;

        let mut resolved_packages = vec![];

        let monorepo_templates =
            resolve_monorepo_templates(&self.toml_config.defaults)?;

        for package in packages {
            let params = PackageResolverParams {
                package_config: package,
                repo_name: &self.repo_name,
                defaults: &self.toml_config.defaults,
                commit_modifiers: &commit_modifiers,
                package_overrides: &self.package_overrides,
                global_overrides: &self.global_overrides,
                compare_link_base_url: &self.compare_link_base_url,
                release_link_base_url: &self.release_link_base_url,
            };

            let resolved_package = resolve_package(params)?;

            resolved_packages.push(resolved_package);
        }

        // If no package renders PR links there is no reason to look any up,
        // so collapse the per-package flags into one switch here.
        let pr_links_enabled = resolved_packages
            .iter()
            .any(|p| p.analyzer_config.include_pr_link);

        let resolved_hash = ResolvedPackageHash::new(resolved_packages)?;

        Ok(Rc::new(ResolvedConfig {
            repo_name: self.repo_name.clone(),
            base_branch,
            package_configs: resolved_hash,
            separate_pull_requests,
            pr_links_enabled,
            monorepo_commit_message_template: monorepo_templates.commit_message,
            monorepo_pr_title_template: monorepo_templates.pr_title,
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::config::{
        changelog::ChangelogConfig, defaults::DefaultsConfig,
        package::PackageConfig,
    };

    use super::*;

    fn resolver(toml_config: Config) -> Resolver {
        Resolver::builder()
            .toml_config(Rc::new(toml_config))
            .repo_name("test-repo")
            .repo_default_branch("main")
            .release_link_base_url(Url::parse("https://example.com/").unwrap())
            .compare_link_base_url(
                Url::parse("https://example.com/compare/").unwrap(),
            )
            .package_overrides(HashMap::new())
            .global_overrides(GlobalOverrides::default())
            .commit_modifiers(CommitModifiers::default())
            .build()
            .unwrap()
    }

    fn package(name: &str) -> PackageConfig {
        PackageConfig {
            name: name.into(),
            ..PackageConfig::default()
        }
    }

    fn changelog_with_pr_link(include_pr_link: bool) -> ChangelogConfig {
        ChangelogConfig {
            include_pr_link: Some(include_pr_link),
            ..ChangelogConfig::default()
        }
    }

    /// Builds a package that sets `include_pr_link` explicitly, rather than
    /// inheriting it from `[defaults]`.
    fn package_with_pr_links(name: &str, enabled: bool) -> PackageConfig {
        PackageConfig {
            changelog: Some(changelog_with_pr_link(enabled)),
            ..package(name)
        }
    }

    fn resolve_pr_links(
        defaults: Option<bool>,
        packages: Vec<PackageConfig>,
    ) -> bool {
        let resolver = resolver(Config {
            defaults: DefaultsConfig {
                changelog: defaults.map(changelog_with_pr_link),
                ..DefaultsConfig::default()
            },
            ..Config::default()
        });

        resolver.resolve(packages).unwrap().pr_links_enabled
    }

    /// `pr_links_enabled` is the single switch that decides whether the
    /// forge is asked for PR links at all. It is derived from the *resolved*
    /// per-package values, so it accounts for defaults merging and CLI
    /// overrides rather than re-reading the raw TOML.
    #[test]
    fn pr_links_disabled_when_nothing_opts_in() {
        assert!(!resolve_pr_links(None, vec![package("a")]));
    }

    #[test]
    fn pr_links_enabled_when_inherited_from_defaults() {
        // The package sets no `changelog` at all, so it inherits the default.
        assert!(resolve_pr_links(Some(true), vec![package("a")]));
    }

    #[test]
    fn pr_links_enabled_when_a_single_package_opts_in() {
        // Commits are enriched per package, but one package opting in is
        // enough to make the lookups necessary.
        assert!(resolve_pr_links(
            None,
            vec![package("a"), package_with_pr_links("b", true), package("c"),]
        ));
    }

    #[test]
    fn pr_links_disabled_when_explicitly_off_everywhere() {
        assert!(!resolve_pr_links(
            Some(false),
            vec![package_with_pr_links("a", false)]
        ));
    }

    /// A package's `changelog` merges field-by-field over `[defaults]`, so an
    /// explicit `false` wins. With every package opted out there is nothing
    /// left to render links for, so no lookups should happen either.
    #[test]
    fn pr_links_disabled_when_defaults_on_but_every_package_opts_out() {
        assert!(!resolve_pr_links(
            Some(true),
            vec![
                package_with_pr_links("a", false),
                package_with_pr_links("b", false),
            ]
        ));
    }

    /// The inverse of the above: one package keeping the inherited default is
    /// enough, even when its siblings opt out.
    #[test]
    fn pr_links_enabled_when_one_package_keeps_the_default() {
        assert!(resolve_pr_links(
            Some(true),
            vec![package_with_pr_links("a", false), package("b")]
        ));
    }

    /// `repo_name` and the two monorepo templates live on `ResolvedConfig`
    /// rather than on a package, so nothing else in the pipeline covers
    /// them reaching the other side of resolution.
    #[test]
    fn resolve_carries_repo_name_and_monorepo_templates() {
        let resolver = resolver(Config {
            defaults: DefaultsConfig {
                monorepo_commit_message_template: Some("mono commit".into()),
                monorepo_pr_title_template: Some("mono title".into()),
                commit_message_template: Some("pkg commit".into()),
                pr_title_template: Some("pkg title".into()),
                ..DefaultsConfig::default()
            },
            ..Config::default()
        });

        let resolved = resolver.resolve(vec![package("test-pkg")]).unwrap();

        assert_eq!(resolved.repo_name, "test-repo");
        assert_eq!(resolved.monorepo_commit_message_template, "mono commit");
        assert_eq!(resolved.monorepo_pr_title_template, "mono title");

        let pkg = resolved.package_configs.get("test-pkg").unwrap();

        assert_eq!(pkg.commit_message_template, "pkg commit");
        assert_eq!(pkg.pr_title_template, "pkg title");
    }

    /// Validation runs during resolution, so a bad template stops the
    /// release before any forge call is made.
    #[test]
    fn resolve_rejects_an_invalid_template() {
        let resolver = resolver(Config {
            defaults: DefaultsConfig {
                monorepo_pr_title_template: Some("{{ package_name }}".into()),
                ..DefaultsConfig::default()
            },
            ..Config::default()
        });

        let Err(err) = resolver.resolve(vec![package("test-pkg")]) else {
            panic!("expected the invalid template to be rejected");
        };

        assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));
    }
}
