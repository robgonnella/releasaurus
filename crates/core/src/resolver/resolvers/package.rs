use std::rc::Rc;

use crate::{
    config::{
        VersionType, package::PackageConfig, prerelease::PrereleaseConfig,
        resolved::ResolvedConfig,
    },
    packages::resolved::ResolvedPackage,
    resolver::resolvers::{
        analyzer::{AnalyzerParams, build_analyzer_config},
        auto_start::resolve_auto_start_next,
        manifest::compile_additional_manifests,
        package_name::resolve_package_name,
        path_utils::{normalize_additional_paths, normalize_package_paths},
        prerelease::resolve_prerelease,
        sub_packages::resolve_sub_packages_full,
        tag_prefix::resolve_tag_prefix,
        version_increment::{
            resolve_custom_increment_regexes, resolve_version_increment_flags,
            resolve_version_type,
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

    let version_type = resolve_version_type(
        &name,
        &package_config,
        resolved_config.version_type,
        &resolved_config.global_overrides,
        &resolved_config.package_overrides,
    );

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

    warn_ignored_semantic_config(
        &name,
        version_type,
        prerelease.as_ref(),
        custom_major_increment_regex.as_ref(),
        custom_minor_increment_regex.as_ref(),
        breaking_always_increment_major,
        features_always_increment_minor,
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

    // Build analyzer config
    let analyzer_config = build_analyzer_config(AnalyzerParams {
        config: Rc::clone(&resolved_config),
        package_name: name.clone(),
        prerelease: prerelease.clone(),
        tag_prefix: tag_prefix.clone(),
        version_type,
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
        auto_start_next: auto_start,
        normalized_additional_paths,
        compiled_additional_manifests,
        analyzer_config,
    })
}

/// Warns when semantic-only settings are configured alongside a date-based
/// version type. Prerelease, custom increment regexes, and the increment
/// flags only apply to the major.minor.patch and major.minor.patch+timestamp.sha version types; for
/// date-based types they are silently ignored, so surface that to the user
/// rather than dropping them without notice.
///
/// The increment flags arrive as `Option<bool>` (package config falling back
/// to global config, default not yet applied). A `Some(_)` means the user
/// explicitly set the flag somewhere, which is the case worth warning about;
/// `None` means it was left to the default and is silent.
fn warn_ignored_semantic_config(
    package: &str,
    version_type: VersionType,
    prerelease: Option<&PrereleaseConfig>,
    custom_major_increment_regex: Option<&String>,
    custom_minor_increment_regex: Option<&String>,
    breaking_always_increment_major: Option<bool>,
    features_always_increment_minor: Option<bool>,
) {
    if !version_type.is_date_based() {
        return;
    }

    if prerelease.is_some() {
        log::warn!(
            "package \"{package}\": prerelease config is ignored for \
             version_type {version_type}; prerelease only applies to \
             major.minor.patch and major.minor.patch+timestamp.sha"
        );
    }

    if custom_major_increment_regex.is_some() {
        log::warn!(
            "package \"{package}\": custom_major_increment_regex is ignored \
             for version_type {version_type}; it only applies to major.minor.patch \
             and major.minor.patch+timestamp.sha"
        );
    }

    if custom_minor_increment_regex.is_some() {
        log::warn!(
            "package \"{package}\": custom_minor_increment_regex is ignored \
             for version_type {version_type}; it only applies to major.minor.patch \
             and major.minor.patch+timestamp.sha"
        );
    }

    if breaking_always_increment_major.is_some() {
        log::warn!(
            "package \"{package}\": breaking_always_increment_major is \
             ignored for version_type {version_type}; it only applies to \
             major.minor.patch and major.minor.patch+timestamp.sha"
        );
    }

    if features_always_increment_minor.is_some() {
        log::warn!(
            "package \"{package}\": features_always_increment_minor is \
             ignored for version_type {version_type}; it only applies to \
             major.minor.patch and major.minor.patch+timestamp.sha"
        );
    }
}
