//! Common test utilities for orchestrator core tests.

use semver::Version;
use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};
use url::Url;

use crate::{
    config::{
        Config,
        overrides::{CommitModifiers, GlobalOverrides},
        package::{PackageConfig, PackageConfigBuilder},
    },
    forge::{
        manager::{ForgeManager, ForgeOptions},
        request::{
            Commit, PrMetadataBlock, ResolvedCreateReleaseBranchRequest, Tag,
        },
        traits::MockForge,
    },
    orchestrator::package_processor::PackageProcessor,
    packages::{releasable::ReleasablePackage, release_pr::ReleasePRPackage},
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

/// Stubs metadata encoding with the plain HTML comment form used by
/// GitHub and Gitea.
pub fn expect_html_comment_encoding(mock: &mut MockForge) {
    mock.expect_encode_pr_metadata()
        .returning(|json| PrMetadataBlock {
            inline_content: format!("<!--{json}-->"),
            div_attribute: String::new(),
        });
}

/// Records the commit messages passed to `create_release_branch` so a
/// test can compare them against expected strings instead of asserting
/// inside a `withf` predicate, which reports nothing useful on failure.
///
/// `Arc<Mutex<_>>` rather than the `Rc` used elsewhere in the crate
/// because mockall requires its return closures to be `Send`.
pub fn capture_release_branch_messages(
    mock: &mut MockForge,
    times: usize,
) -> Arc<Mutex<Vec<String>>> {
    let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let sink = Arc::clone(&captured);

    mock.expect_create_release_branch().times(times).returning(
        move |req: ResolvedCreateReleaseBranchRequest| {
            sink.lock().unwrap().push(req.message);
            Ok(Commit {
                sha: "abc123".to_string(),
            })
        },
    );

    captured
}

/// Builds a minimal releasable package with `tag` parsed into a semver.
pub fn releasable(name: &str, tag: &str) -> ReleasablePackage {
    ReleasablePackage {
        name: name.into(),
        tag: tag_for(tag),
        notes: format!("notes for {name}"),
        ..Default::default()
    }
}

/// Builds a release PR package for tests that drive the render helpers
/// directly rather than going through `create_pr_branches`.
pub fn release_pr_package(
    name: &str,
    tag: &str,
    commit_message_template: &str,
    pr_title_template: &str,
) -> ReleasePRPackage {
    ReleasePRPackage {
        name: name.into(),
        tag: tag_for(tag),
        notes: String::new(),
        tag_compare_link: String::new(),
        sha_compare_link: String::new(),
        file_changes: vec![],
        release_branch: "releasaurus-release-main".into(),
        commit_message_template: commit_message_template.into(),
        pr_title_template: pr_title_template.into(),
    }
}

/// Builds a tag from a `v`-prefixed name, parsing the remainder as semver.
fn tag_for(name: &str) -> Tag {
    Tag {
        name: name.into(),
        semver: Version::parse(name.trim_start_matches('v')).unwrap(),
        ..Default::default()
    }
}
