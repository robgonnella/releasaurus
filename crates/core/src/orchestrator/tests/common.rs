//! Common test utilities for orchestrator tests.

pub use semver::Version;
use std::{collections::HashMap, fmt::Display, rc::Rc};
use url::Url;

use crate::{
    config::{
        Config,
        package::{PackageConfig, PackageConfigBuilder},
        resolved::{CommitModifiers, GlobalOverrides},
    },
    forge::{
        manager::{ForgeManager, ForgeOptions},
        traits::MockForge,
    },
    orchestrator::{Orchestrator, OrchestratorParams},
    resolver::ResolverBuilder,
};

pub const TEST_PKG_NAME: &str = "test-pkg";

/// Input for make_pr_body helper
pub(crate) struct PrBodyInput<S: Display> {
    pub(crate) pkg: S,
    pub(crate) tag: S,
    pub(crate) notes: S,
    pub(crate) tag_link: S,
    pub(crate) sha_link: S,
    pub(crate) header: S,
    pub(crate) footer: S,
}

/// Builds a PR body in the new HTML format
pub(crate) fn make_pr_body<S: Display>(input: &PrBodyInput<S>) -> String {
    let json = format!(
        r#"{{"metadata":{{"sha_compare_link":"{}","tag_compare_link":"{}"}}}}"#,
        input.sha_link, input.tag_link
    );
    format!(
        r#"<details open>
<summary>{}</summary>
<div id="{}-header">{}</div>
<div id="{}" data-tag="{}">
<!--{json}-->

{}
</div>
<div id="{}-footer">{}</div>
</details>"#,
        input.tag,
        input.pkg,
        input.header,
        input.pkg,
        input.tag,
        input.notes,
        input.pkg,
        input.footer
    )
}

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

    let resolver = ResolverBuilder::default()
        .toml_config(config)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url(Url::parse("file:///").unwrap())
        .compare_link_base_url(Url::parse("file:///").unwrap())
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let pkg_config = PackageConfigBuilder::default()
        .name(TEST_PKG_NAME)
        .path(".")
        .build()
        .unwrap();

    let (resolved_config, resolved) =
        resolver.resolve(vec![pkg_config]).unwrap();

    let forge = Rc::new(ForgeManager::new(
        Box::new(mock_forge),
        ForgeOptions { dry_run: false },
    ));

    Orchestrator::new(OrchestratorParams {
        config: resolved_config,
        package_configs: Rc::new(resolved),
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

    let resolver = ResolverBuilder::default()
        .toml_config(config_rc)
        .repo_name("test-repo")
        .repo_default_branch("main")
        .release_link_base_url(Url::parse("file:///").unwrap())
        .compare_link_base_url(Url::parse("file:///").unwrap())
        .package_overrides(HashMap::new())
        .global_overrides(GlobalOverrides::default())
        .commit_modifiers(CommitModifiers::default())
        .build()
        .unwrap();

    let (resolved_config, resolved_pkgs) = resolver.resolve(packages).unwrap();

    let forge = Rc::new(ForgeManager::new(
        Box::new(mock_forge),
        ForgeOptions { dry_run: false },
    ));

    Orchestrator::new(OrchestratorParams {
        config: resolved_config,
        package_configs: Rc::new(resolved_pkgs),
        forge,
    })
    .unwrap()
}
