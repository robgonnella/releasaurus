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
    /// Branch release PRs target and commits are read from.
    pub base_branch: String,
    /// Whether each package gets its own release PR.
    pub separate_pull_requests: bool,
    /// Resolved per-package config, indexed by package name.
    pub package_configs: ResolvedPackageHash,
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

        let default_versioning = self.toml_config.defaults.versioning.as_ref();

        let default_changelog = self
            .toml_config
            .defaults
            .changelog
            .clone()
            .unwrap_or_default();

        let separate_pull_requests =
            self.toml_config.repository.separate_pull_requests;

        let mut resolved_packages = vec![];

        for package in packages {
            let params = PackageResolverParams {
                package_config: package,
                repo_name: &self.repo_name,
                default_versioning,
                default_changelog: &default_changelog,
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
            base_branch,
            package_configs: resolved_hash,
            separate_pull_requests,
        }))
    }
}
