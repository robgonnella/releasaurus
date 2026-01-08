//! Common test utilities for orchestrator core tests.

use std::rc::Rc;

use crate::{
    OrchestratorConfig, ResolvedPackage,
    cli::{CommitModifiers, GlobalOverrides},
    config::{
        Config,
        package::{PackageConfig, PackageConfigBuilder},
    },
    forge::{manager::ForgeManager, traits::MockForge},
    orchestrator::{core::Core, package::resolved::ResolvedPackageHash},
};

pub use semver::Version;

/// Creates a Core instance with the provided mock forge, optional package
/// configs, and optional config. This allows tests to set expectations on the
/// mock before creating the core.
pub fn create_core(
    mock_forge: MockForge,
    pkg_configs: Option<Vec<PackageConfig>>,
    config: Option<Config>,
) -> Core {
    let config = Rc::new(config.unwrap_or_default());

    let orchestrator_config = Rc::new(
        OrchestratorConfig::builder()
            .toml_config(config)
            .repo_name("test-repo")
            .repo_default_branch("main")
            .release_link_base_url("https://example.com")
            .package_overrides(std::collections::HashMap::new())
            .global_overrides(GlobalOverrides::default())
            .commit_modifiers(CommitModifiers::default())
            .build()
            .unwrap(),
    );

    let forge = Rc::new(ForgeManager::new(Box::new(mock_forge)));

    let pkg_configs = pkg_configs.unwrap_or(vec![
        PackageConfigBuilder::default()
            .name("test-pkg")
            .path(".")
            .build()
            .unwrap(),
    ]);

    let resolved = pkg_configs
        .into_iter()
        .map(|p| {
            ResolvedPackage::builder()
                .orchestrator_config(Rc::clone(&orchestrator_config))
                .package_config(p)
                .build()
                .unwrap()
        })
        .collect();

    let package_configs = Rc::new(ResolvedPackageHash::new(resolved).unwrap());

    Core::new(orchestrator_config, forge, package_configs)
}

/// Re-export commonly used types for tests
pub use crate::orchestrator::package::{
    prepared::PreparedPackage, releasable::ReleasablePackage,
};
