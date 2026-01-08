//! Common test utilities for orchestrator tests.

use std::{collections::HashMap, rc::Rc};

use crate::{
    Orchestrator, OrchestratorConfig, ResolvedPackage,
    cli::{CommitModifiers, GlobalOverrides},
    config::{
        Config,
        package::{PackageConfig, PackageConfigBuilder},
    },
    forge::{manager::ForgeManager, traits::MockForge},
    orchestrator::OrchestratorParams,
};

pub use semver::Version;

pub const TEST_PKG_NAME: &str = "test-pkg";

/// Creates a test Orchestrator with the provided mock forge.
/// This allows tests to set expectations on the mock before creating the manager.
///
/// # Example
/// ```ignore
/// let mut mock_forge = MockForge::new();
/// mock_forge.expect_get_commits().returning(|_, _| Ok(vec![]));
/// let manager = create_test_orchestrator(mock_forge);
/// ```
pub fn create_test_orchestrator(mock_forge: MockForge) -> Orchestrator {
    let config = Rc::new(Config::default());
    let orchestrator_config = Rc::new(
        OrchestratorConfig::builder()
            .toml_config(config)
            .repo_name("test-repo")
            .repo_default_branch("main")
            .release_link_base_url("")
            .package_overrides(HashMap::new())
            .global_overrides(GlobalOverrides::default())
            .commit_modifiers(CommitModifiers::default())
            .build()
            .unwrap(),
    );

    let pkg_config = PackageConfigBuilder::default()
        .name(TEST_PKG_NAME)
        .path(".")
        .build()
        .unwrap();

    let resolved = ResolvedPackage::builder()
        .orchestrator_config(Rc::clone(&orchestrator_config))
        .package_config(pkg_config)
        .build()
        .unwrap();

    let package_configs = Rc::new(
        crate::orchestrator::package::resolved::ResolvedPackageHash::new(vec![
            resolved,
        ])
        .unwrap(),
    );

    let forge = Rc::new(ForgeManager::new(Box::new(mock_forge)));

    Orchestrator::new(OrchestratorParams {
        config: orchestrator_config,
        package_configs,
        forge,
    })
    .unwrap()
}

/// Creates a test Orchestrator with custom configuration.
/// Useful for tests that need specific package setups.
///
/// Note: Pass already-built PackageConfig objects, not builders.
pub fn create_test_orchestrator_with_config(
    mock_forge: MockForge,
    packages: Vec<PackageConfig>,
    config: Option<Config>,
) -> Orchestrator {
    let config_rc = Rc::new(config.unwrap_or_default());

    let orchestrator_config = Rc::new(
        OrchestratorConfig::builder()
            .toml_config(Rc::clone(&config_rc))
            .repo_name("test-repo")
            .repo_default_branch("main")
            .release_link_base_url("https://example.com")
            .package_overrides(HashMap::new())
            .global_overrides(GlobalOverrides::default())
            .commit_modifiers(CommitModifiers::default())
            .build()
            .unwrap(),
    );

    let resolved_packages: Vec<ResolvedPackage> = packages
        .into_iter()
        .map(|pkg_config| {
            ResolvedPackage::builder()
                .orchestrator_config(Rc::clone(&orchestrator_config))
                .package_config(pkg_config)
                .build()
                .unwrap()
        })
        .collect();

    let package_configs = Rc::new(
        crate::orchestrator::package::resolved::ResolvedPackageHash::new(
            resolved_packages,
        )
        .unwrap(),
    );

    let forge = Rc::new(ForgeManager::new(Box::new(mock_forge)));

    Orchestrator::new(OrchestratorParams {
        config: orchestrator_config,
        package_configs,
        forge,
    })
    .unwrap()
}
