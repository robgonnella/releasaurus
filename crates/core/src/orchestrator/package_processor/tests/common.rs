//! Common test utilities for orchestrator core tests.

use std::rc::Rc;
use url::Url;

use crate::{
    config::{
        Config,
        overrides::{CommitModifiers, GlobalOverrides},
        package::{PackageConfig, PackageConfigBuilder},
    },
    forge::{
        manager::{ForgeManager, ForgeOptions},
        traits::MockForge,
    },
    orchestrator::package_processor::PackageProcessor,
    resolver::Resolver,
};

/// Creates a PackageProcessor instance with the provided mock forge, optional
/// package configs, and optional config. This allows tests to set expectations
/// on the mock before creating the core.
pub fn create_package_processor(
    mock_forge: MockForge,
    pkg_configs: Option<Vec<PackageConfig>>,
    config: Option<Config>,
) -> PackageProcessor {
    let config = Rc::new(config.unwrap_or_default());

    let resolver = Resolver::builder()
        .toml_config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url(Url::parse("https://example.com/").unwrap())
        .compare_link_base_url(
            Url::parse("https://example.com/compare/").unwrap(),
        )
        .package_overrides(std::collections::HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let forge = Rc::new(ForgeManager::new(
        Box::new(mock_forge),
        ForgeOptions { dry_run: false },
    ));

    let pkg_configs = pkg_configs.unwrap_or(vec![
        PackageConfigBuilder::default()
            .name("test-pkg")
            .path(".")
            .build()
            .unwrap(),
    ]);

    let resolved_config = resolver.resolve(pkg_configs).unwrap();

    PackageProcessor::new(resolved_config, forge)
}
