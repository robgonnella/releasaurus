use url::Url;

use crate::{
    config::{
        changelog::{ChangelogConfig, DEFAULT_AGGREGATE_PRERELEASES},
        overrides::{CommitModifiers, GlobalOverrides, PackageOverridesHash},
        package::PackageConfig,
        versioning::VersioningConfig,
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

pub struct PackageResolverParams<'a> {
    pub package_config: PackageConfig,
    pub repo_name: &'a str,
    pub default_versioning: Option<&'a VersioningConfig>,
    pub default_changelog: &'a ChangelogConfig,
    pub commit_modifiers: &'a CommitModifiers,
    pub package_overrides: &'a PackageOverridesHash,
    pub global_overrides: &'a GlobalOverrides,
    pub compare_link_base_url: &'a Url,
    pub release_link_base_url: &'a Url,
}

pub fn resolve_package(
    params: PackageResolverParams,
) -> Result<ResolvedPackage> {
    let PackageResolverParams {
        package_config,
        repo_name,
        default_versioning,
        default_changelog,
        commit_modifiers,
        package_overrides,
        global_overrides,
        compare_link_base_url,
        release_link_base_url,
    } = params;

    let name = resolve_package_name(&package_config, repo_name);

    let tag_prefix = resolve_tag_prefix(
        &name,
        &package_config,
        package_overrides,
        global_overrides,
    );

    let versioning_config = resolve_versioning(
        &package_config,
        default_versioning,
        package_overrides,
        global_overrides,
    )?;

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
        resolve_changelog_config(&package_config, default_changelog);

    let aggregate_prereleases = changelog_config
        .aggregate_prereleases
        .unwrap_or(DEFAULT_AGGREGATE_PRERELEASES);

    // Build analyzer config
    let analyzer_config = build_analyzer_config(AnalyzerParams {
        changelog: changelog_config,
        versioning: versioning_config.clone(),
        commit_modifiers: commit_modifiers.clone(),
        compare_link_base_url: Some(compare_link_base_url.clone()),
        release_link_base_url: Some(release_link_base_url.clone()),
        tag_prefix: tag_prefix.clone(),
    });

    let release_type = package_config.release_type.unwrap_or_default();

    // Resolve sub-packages
    let sub_packages = resolve_sub_packages_full(
        package_config,
        repo_name,
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
