use derive_builder::Builder;
use std::{collections::HashMap, rc::Rc};
use url::Url;

use crate::{
    config::{
        Config,
        package::PackageConfig,
        resolved::{
            CommitModifiers, GlobalOverrides, PackageOverrides, ResolvedConfig,
        },
    },
    packages::resolved_hash::ResolvedPackageHash,
    resolver::resolvers::{
        base_branch::resolve_base_branch,
        commit_modifiers::resolve_commit_modifiers, package::resolve_package,
    },
    result::{ReleasaurusError, Result},
};

pub mod resolvers;

#[derive(Builder)]
#[builder(setter(into), build_fn(private, name = "_build"))]
pub struct Resolver {
    pub toml_config: Rc<Config>,
    pub repo_name: String,
    pub repo_default_branch: String,
    pub release_link_base_url: Url,
    pub compare_link_base_url: Url,
    pub package_overrides: HashMap<String, PackageOverrides>,
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
    ) -> Result<(Rc<ResolvedConfig>, ResolvedPackageHash)> {
        let base_branch = resolve_base_branch(
            &self.toml_config,
            &self.global_overrides,
            &self.repo_default_branch,
        );

        let commit_modifiers = resolve_commit_modifiers(
            &self.toml_config,
            &self.commit_modifiers,
        )?;

        let resolved_config = Rc::new(ResolvedConfig {
            auto_start_next: self.toml_config.auto_start_next,
            base_branch,
            breaking_always_increment_major: self
                .toml_config
                .breaking_always_increment_major,
            changelog: self.toml_config.changelog.clone(),
            commit_modifiers,
            custom_major_increment_regex: self
                .toml_config
                .custom_major_increment_regex
                .clone(),
            custom_minor_increment_regex: self
                .toml_config
                .custom_minor_increment_regex
                .clone(),
            features_always_increment_minor: self
                .toml_config
                .features_always_increment_minor,
            first_release_search_depth: self
                .toml_config
                .first_release_search_depth,
            global_overrides: self.global_overrides.clone(),
            package_overrides: self.package_overrides.clone(),
            prerelease: self.toml_config.prerelease.clone(),
            release_link_base_url: self.release_link_base_url.clone(),
            compare_link_base_url: self.compare_link_base_url.clone(),
            repo_name: self.repo_name.clone(),
            separate_pull_requests: self.toml_config.separate_pull_requests,
        });

        let mut resolved_packages = vec![];

        for package in packages {
            let resolved_package =
                resolve_package(Rc::clone(&resolved_config), package)?;

            resolved_packages.push(resolved_package);
        }

        let resolved_hash = ResolvedPackageHash::new(resolved_packages)?;

        Ok((resolved_config, resolved_hash))
    }
}
