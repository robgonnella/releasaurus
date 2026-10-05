use chrono::{TimeZone, Utc};
use url::Url;

use crate::{
    config::{
        changelog::DEFAULT_AGGREGATE_PRERELEASES,
        defaults::DefaultsConfig,
        overrides::{CommitModifiers, GlobalOverrides, PackageOverridesHash},
        package::PackageConfig,
        release_type::ReleaseType,
        versioning::{DEFAULT_VERSION_TYPE, VersionType, VersioningConfig},
    },
    forge::link::LinkStyle,
    packages::resolved::ResolvedPackage,
    resolver::resolvers::{
        analyzer::{AnalyzerParams, build_analyzer_config},
        changelog::resolve_changelog_config,
        manifest::resolve_additional_manifests,
        package_name::resolve_package_name,
        path_utils::{normalize_additional_paths, normalize_package_paths},
        sub_packages::resolve_sub_packages_full,
        tag_prefix::resolve_tag_prefix,
        templates::resolve_package_templates,
        versioning::resolve_versioning,
    },
    result::Result,
};

pub struct PackageResolverParams<'a> {
    pub package_config: PackageConfig,
    pub repo_name: &'a str,
    pub defaults: &'a DefaultsConfig,
    pub commit_modifiers: &'a CommitModifiers,
    pub package_overrides: &'a PackageOverridesHash,
    pub global_overrides: &'a GlobalOverrides,
    pub compare_link_base_url: &'a Url,
    pub release_link_base_url: &'a Url,
    pub link_style: LinkStyle,
}

pub fn resolve_package(
    params: PackageResolverParams,
) -> Result<ResolvedPackage> {
    let PackageResolverParams {
        package_config,
        repo_name,
        defaults,
        commit_modifiers,
        package_overrides,
        global_overrides,
        compare_link_base_url,
        release_link_base_url,
        link_style,
    } = params;

    let name = resolve_package_name(&package_config, repo_name);

    let tag_prefix = resolve_tag_prefix(
        &name,
        &package_config,
        package_overrides,
        global_overrides,
    );

    let versioning_config = resolve_versioning(
        &name,
        &package_config,
        defaults.versioning.as_ref(),
        package_overrides,
        global_overrides,
    )?;

    warn_ignored_semantic_config(
        &name,
        &versioning_config,
        package_config.versioning.as_ref(),
    );

    // Normalize paths
    let (normalized_workspace_root, normalized_full_path) =
        normalize_package_paths(&package_config);

    // Additional manifests for regex based version replacement
    let additional_manifests =
        resolve_additional_manifests(&normalized_full_path, &package_config)?;

    // Resolve additional paths
    let normalized_additional_paths =
        normalize_additional_paths(&package_config);

    let default_changelog = defaults.changelog.clone().unwrap_or_default();

    let changelog_config =
        resolve_changelog_config(&package_config, &default_changelog);

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
        link_style,
        tag_prefix: tag_prefix.clone(),
    });

    let release_type = package_config.release_type.unwrap_or_default();

    let templates =
        resolve_package_templates(&name, &package_config, defaults)?;

    // Resolve sub-packages
    let sub_packages = resolve_sub_packages_full(
        package_config,
        repo_name,
        &normalized_workspace_root,
        &tag_prefix,
        &analyzer_config,
        &versioning_config,
    );

    if let Some(VersionType::Strftime(fmt)) = &versioning_config.version_type
        && let Some(sample) = non_semver_sample(fmt)
    {
        let packages = std::iter::once((name.as_str(), release_type)).chain(
            sub_packages
                .iter()
                .map(|s| (s.name.as_str(), s.release_type)),
        );
        for (package, release_type) in packages {
            if matches!(
                release_type,
                ReleaseType::Rust | ReleaseType::Node | ReleaseType::Go
            ) {
                log::warn!(
                    "package \"{package}\": version_type {fmt} produces \
                     versions such as {sample} that are not semver; \
                     {release_type} tooling will reject them in the manifest"
                );
            }
        }
    }

    Ok(ResolvedPackage {
        name,
        normalized_workspace_root,
        normalized_full_path,
        release_type,
        tag_prefix,
        sub_packages,
        aggregate_prereleases,
        normalized_additional_paths,
        additional_manifests,
        analyzer_config,
        versioning_config,
        commit_message_template: templates.commit_message,
        pr_title_template: templates.pr_title,
    })
}

/// What a strftime `version_type` renders, when that is not semver. The
/// fields are single-digit so zero padding, which semver forbids, shows up
/// regardless of today's date.
fn non_semver_sample(fmt: &str) -> Option<String> {
    let sample = Utc
        .with_ymd_and_hms(2026, 1, 2, 3, 4, 5)
        .unwrap()
        .format(fmt)
        .to_string();
    semver::Version::parse(&sample).is_err().then_some(sample)
}

/// Warns when semantic-only settings are configured alongside a date-based
/// version type. Prerelease, the custom increment regexes, and the increment
/// flags only apply to the two `major.minor.patch` version types; for
/// date-based types they are silently ignored, so surface that to the user
/// rather than dropping them without notice.
///
/// Which tier is inspected differs per setting. Prerelease and the custom
/// regexes are never part of the documented baseline config, so an inherited
/// `[defaults.versioning]` value is still worth reporting and they are read
/// from `resolved`. The increment flags *are* part of it — the configuration
/// reference recommends setting both globally — so they are read from
/// `package_versioning` and only reported when set on this package, keeping
/// one date-based package from warning about a repo-wide default.
fn warn_ignored_semantic_config(
    package: &str,
    resolved: &VersioningConfig,
    package_versioning: Option<&VersioningConfig>,
) {
    let version_type = resolved
        .version_type
        .as_ref()
        .unwrap_or(&DEFAULT_VERSION_TYPE);

    if !version_type.is_date_based() {
        return;
    }

    let ignored = [
        ("prerelease", resolved.prerelease.is_some()),
        (
            "custom_major_increment_regex",
            resolved.custom_major_increment_regex.is_some(),
        ),
        (
            "custom_minor_increment_regex",
            resolved.custom_minor_increment_regex.is_some(),
        ),
        (
            "breaking_always_increment_major",
            package_versioning
                .is_some_and(|v| v.breaking_always_increment_major.is_some()),
        ),
        (
            "features_always_increment_minor",
            package_versioning
                .is_some_and(|v| v.features_always_increment_minor.is_some()),
        ),
    ];

    for (setting, is_set) in ignored {
        if is_set {
            log::warn!(
                "package \"{package}\": {setting} is ignored for version_type \
                 {version_type}; it only applies to major.minor.patch and \
                 major.minor.patch+timestamp.sha"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_semver_sample_flags_zero_padding_regardless_of_date() {
        // %d only pads on days 1–9, so a sample taken from "now" would
        // miss this most of the month
        assert_eq!(
            non_semver_sample("%Y.%-m.%d").as_deref(),
            Some("2026.1.02")
        );
        assert_eq!(non_semver_sample("%Y.%m").as_deref(), Some("2026.01"));
        assert_eq!(non_semver_sample("%y%m%d").as_deref(), Some("260102"));
    }

    #[test]
    fn non_semver_sample_accepts_semver_shaped_formats() {
        for fmt in ["%Y.%-m.%-d", "%y.%-m.%-d", "%Y.%-m.%-d+%H.%M.%S"] {
            assert_eq!(non_semver_sample(fmt), None, "{fmt:?}");
        }
    }
}
