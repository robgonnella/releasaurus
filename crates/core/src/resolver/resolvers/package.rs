use std::rc::Rc;

use crate::{
    config::{
        changelog::DEFAULT_AGGREGATE_PRERELEASES, package::PackageConfig,
        resolved::ResolvedConfig,
    },
    packages::resolved::ResolvedPackage,
    resolver::resolvers::{
        analyzer::{AnalyzerParams, build_analyzer_config},
        auto_start::resolve_auto_start_next,
        changelog::resolve_changelog_config,
        manifest::compile_additional_manifests,
        package_name::resolve_package_name,
        path_utils::{normalize_additional_paths, normalize_package_paths},
        prerelease::resolve_prerelease,
        sub_packages::resolve_sub_packages_full,
        tag_prefix::resolve_tag_prefix,
        version_increment::{
            resolve_custom_increment_regexes, resolve_version_increment_flags,
        },
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
    let auto_start = resolve_auto_start_next(
        &package_config,
        resolved_config.auto_start_next,
    );

    // Resolve complex configurations
    let prerelease = resolve_prerelease(
        &package_config,
        &resolved_config.prerelease,
        &resolved_config.global_overrides,
        &resolved_config.package_overrides,
    )?;

    let (breaking_always_increment_major, features_always_increment_minor) =
        resolve_version_increment_flags(
            &package_config,
            resolved_config.breaking_always_increment_major,
            resolved_config.features_always_increment_minor,
        );

    let (custom_major_increment_regex, custom_minor_increment_regex) =
        resolve_custom_increment_regexes(
            &package_config,
            &resolved_config.custom_major_increment_regex,
            &resolved_config.custom_minor_increment_regex,
        );

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
        changelog_config,
        commit_modifiers: resolved_config.commit_modifiers.clone(),
        compare_link_base_url: Some(
            resolved_config.compare_link_base_url.clone(),
        ),
        release_link_base_url: Some(
            resolved_config.release_link_base_url.clone(),
        ),
        package_name: name.clone(),
        prerelease: prerelease.clone(),
        tag_prefix: tag_prefix.clone(),
        breaking_always_increment_major,
        custom_major_increment_regex,
        features_always_increment_minor,
        custom_minor_increment_regex,
    });

    let release_type = package_config.release_type.unwrap_or_default();

    // Resolve sub-packages
    let sub_packages = resolve_sub_packages_full(
        Rc::clone(&resolved_config),
        package_config,
        &normalized_workspace_root,
        &tag_prefix,
        prerelease.clone(),
        auto_start,
        &analyzer_config,
    );

    Ok(ResolvedPackage {
        name,
        normalized_workspace_root,
        normalized_full_path,
        release_type,
        tag_prefix,
        sub_packages,
        prerelease,
        aggregate_prereleases,
        auto_start_next: auto_start,
        normalized_additional_paths,
        compiled_additional_manifests,
        analyzer_config,
    })
}
