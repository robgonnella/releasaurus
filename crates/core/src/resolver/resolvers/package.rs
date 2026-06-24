use std::rc::Rc;

use crate::{
    config::{
        changelog::DEFAULT_AGGREGATE_PRERELEASES, package::PackageConfig,
        resolved::ResolvedConfig,
    },
    packages::resolved::ResolvedPackage,
    resolver::resolvers::{
        analyzer::{AnalyzerParams, build_analyzer_config},
        changelog::resolve_changelog_config,
        manifest::compile_additional_manifests,
        package_name::resolve_package_name,
        path_utils::{normalize_additional_paths, normalize_package_paths},
        sub_packages::resolve_sub_packages_full,
        tag_prefix::resolve_tag_prefix,
        versioning::resolve_versioning,
    },
    result::Result,
};

pub fn resolve_package(
    resolved_config: Rc<ResolvedConfig>,
    package_config: PackageConfig,
) -> Result<ResolvedPackage> {
    let name =
        resolve_package_name(&package_config, &resolved_config.repo_name);

    let tag_prefix = resolve_tag_prefix(
        &name,
        &package_config,
        &resolved_config.package_overrides,
        &resolved_config.global_overrides,
    );

    let versioning_config =
        resolve_versioning(&resolved_config, &package_config)?;

    // Normalize paths
    let (normalized_workspace_root, normalized_full_path) =
        normalize_package_paths(&package_config);

    // Compile manifests
    let compiled_additional_manifests =
        compile_additional_manifests(&normalized_full_path, &package_config)?;

    // Resolve additional paths
    let normalized_additional_paths =
        normalize_additional_paths(&package_config);

    let changelog_config =
        resolve_changelog_config(&package_config, &resolved_config.changelog);

    let aggregate_prereleases = changelog_config
        .aggregate_prereleases
        .unwrap_or(DEFAULT_AGGREGATE_PRERELEASES);

    // Build analyzer config
    let analyzer_config = build_analyzer_config(AnalyzerParams {
        changelog: changelog_config,
        versioning: versioning_config.clone(),
        commit_modifiers: resolved_config.commit_modifiers.clone(),
        compare_link_base_url: Some(
            resolved_config.compare_link_base_url.clone(),
        ),
        release_link_base_url: Some(
            resolved_config.release_link_base_url.clone(),
        ),
        tag_prefix: tag_prefix.clone(),
    });

    let release_type = package_config.release_type.unwrap_or_default();

    // Resolve sub-packages
    let sub_packages = resolve_sub_packages_full(
        Rc::clone(&resolved_config),
        package_config,
        &normalized_workspace_root,
        &tag_prefix,
        &analyzer_config,
        &versioning_config,
    );

    Ok(ResolvedPackage {
        name,
        normalized_workspace_root,
        normalized_full_path,
        release_type,
        tag_prefix,
        sub_packages,
        aggregate_prereleases,
        normalized_additional_paths,
        compiled_additional_manifests,
        analyzer_config,
        versioning_config,
    })
}
