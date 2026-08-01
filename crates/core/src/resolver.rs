use derive_builder::Builder;
use std::rc::Rc;
use url::Url;

use crate::{
    config::{
        Config,
        overrides::{CommitModifiers, GlobalOverrides, PackageOverridesHash},
        package::PackageConfig,
    },
    forge::config::DEFAULT_PR_BRANCH_PREFIX,
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
}

impl ResolvedConfig {
    /// The release branch a package's PR lives on. Shared by every
    /// package unless `separate_pull_requests` is set.
    pub fn release_branch_for(&self, package_name: &str) -> String {
        let branch = format!("{DEFAULT_PR_BRANCH_PREFIX}-{}", self.base_branch);

        if self.separate_pull_requests {
            return format!("{branch}-{package_name}");
        }

        branch
    }
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

        let resolved_hash = ResolvedPackageHash::new(resolved_packages)?;

        Ok(Rc::new(ResolvedConfig {
            repo_name: self.repo_name.clone(),
            base_branch,
            package_configs: resolved_hash,
            separate_pull_requests,
            monorepo_commit_message_template: monorepo_templates.commit_message,
            monorepo_pr_title_template: monorepo_templates.pr_title,
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::config::{defaults::DefaultsConfig, package::PackageConfig};

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
