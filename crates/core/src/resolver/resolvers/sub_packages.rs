use std::{path::Path, rc::Rc};

use crate::{
    analyzer::config::AnalyzerConfig,
    config::{
        package::PackageConfig, prerelease::PrereleaseConfig,
        resolved::ResolvedConfig,
    },
    packages::resolved::ResolvedPackage,
    resolver::resolvers::{
        package_name::resolve_sub_package_name, path_utils::normalize_path,
    },
};

/// Resolves all sub-packages for a package.
pub fn resolve_sub_packages_full(
    resolved_config: Rc<ResolvedConfig>,
    package_config: PackageConfig,
    normalized_workspace_root: &Path,
    tag_prefix: &str,
    prerelease: Option<PrereleaseConfig>,
    auto_start: bool,
    analyzer_config: &AnalyzerConfig,
) -> Vec<ResolvedPackage> {
    let PackageConfig {
        sub_packages,
        workspace_root,
        ..
    } = package_config;
    let sub_packages = sub_packages.unwrap_or_default();

    sub_packages
        .iter()
        .map(|s| {
            let name = resolve_sub_package_name(
                s,
                &workspace_root,
                &resolved_config.repo_name,
            );

            let sub_path = normalized_workspace_root
                .join(&s.path)
                .to_string_lossy()
                .to_string();

            let normalized_sub_full = normalize_path(&sub_path);
            let normalized_sub_full_path =
                Path::new(normalized_sub_full.as_ref()).to_path_buf();

            ResolvedPackage {
                name,
                normalized_workspace_root: normalized_workspace_root
                    .to_path_buf(),
                normalized_full_path: normalized_sub_full_path,
                release_type: s.release_type.unwrap_or_default(),
                tag_prefix: tag_prefix.to_string(),
                sub_packages: vec![],
                prerelease: prerelease.clone(),
                auto_start_next: auto_start,
                normalized_additional_paths: vec![],
                compiled_additional_manifests: vec![],
                analyzer_config: analyzer_config.clone(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use url::Url;

    use crate::config::{
        DEFAULT_COMMIT_SEARCH_DEPTH, DEFAULT_TAG_SEARCH_DEPTH,
        changelog::ChangelogConfig,
        package::{PackageConfigBuilder, SubPackage},
        resolved::{CommitModifiers, GlobalOverrides},
    };

    use super::*;

    fn make_resolved_config(name: &str) -> ResolvedConfig {
        ResolvedConfig {
            repo_name: name.to_string(),
            base_branch: "main".into(),
            release_link_base_url: Url::parse("https://example.com/").unwrap(),
            compare_link_base_url: Url::parse("https://example.com/compare/")
                .unwrap(),
            package_overrides: HashMap::default(),
            global_overrides: GlobalOverrides::default(),
            commit_modifiers: CommitModifiers::default(),
            first_release_search_depth: DEFAULT_COMMIT_SEARCH_DEPTH,
            tag_search_depth: DEFAULT_TAG_SEARCH_DEPTH,
            separate_pull_requests: true,
            prerelease: PrereleaseConfig::default(),
            auto_start_next: None,
            breaking_always_increment_major: true,
            features_always_increment_minor: true,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
            changelog: ChangelogConfig::default(),
        }
    }

    #[test]
    fn resolves_sub_packages_with_explicit_names() {
        let resolved_config = Rc::new(make_resolved_config("test-repo"));

        let pkg_config = PackageConfigBuilder::default()
            .name("parent-pkg")
            .path(".")
            .sub_packages(vec![
                SubPackage {
                    name: "sub-pkg-a".to_string(),
                    path: "packages/a".to_string(),
                    ..Default::default()
                },
                SubPackage {
                    name: "sub-pkg-b".to_string(),
                    path: "packages/b".to_string(),
                    ..Default::default()
                },
            ])
            .build()
            .unwrap();

        let workspace_root = Path::new(".");
        let tag_prefix = "v";
        let auto_start = false;
        let prerelease = None;
        let analyzer_config = AnalyzerConfig::default();

        let resolved = resolve_sub_packages_full(
            resolved_config,
            pkg_config,
            workspace_root,
            tag_prefix,
            prerelease,
            auto_start,
            &analyzer_config,
        );

        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].name, "sub-pkg-a");
        assert_eq!(resolved[1].name, "sub-pkg-b");
    }

    #[test]
    fn resolves_sub_packages_with_auto_generated_names() {
        let resolved_config = Rc::new(make_resolved_config("test-repo"));

        let pkg_config = PackageConfigBuilder::default()
            .name("parent-pkg")
            .path(".")
            .sub_packages(vec![SubPackage {
                name: "".to_string(),
                path: "packages/my-package".to_string(),
                ..Default::default()
            }])
            .build()
            .unwrap();

        let workspace_root = Path::new(".");
        let tag_prefix = "v";
        let auto_start = false;
        let prerelease = None;
        let analyzer_config = AnalyzerConfig::default();

        let resolved = resolve_sub_packages_full(
            resolved_config,
            pkg_config,
            workspace_root,
            tag_prefix,
            prerelease,
            auto_start,
            &analyzer_config,
        );

        assert_eq!(resolved.len(), 1);
        // Name should be derived from the last path component
        assert_eq!(resolved[0].name, "my-package");
    }

    #[test]
    fn sub_packages_inherit_parent_tag_prefix() {
        let resolved_config = Rc::new(make_resolved_config("test-repo"));

        let pkg_config = PackageConfigBuilder::default()
            .name("parent-pkg")
            .path(".")
            .tag_prefix("v")
            .sub_packages(vec![SubPackage {
                name: "sub-pkg".to_string(),
                path: "packages/sub".to_string(),
                ..Default::default()
            }])
            .build()
            .unwrap();

        let workspace_root = Path::new(".");
        let expected_tag_prefix = "v";
        let auto_start = false;
        let prerelease = None;
        let analyzer_config = AnalyzerConfig::default();

        let resolved = resolve_sub_packages_full(
            resolved_config,
            pkg_config,
            workspace_root,
            expected_tag_prefix,
            prerelease,
            auto_start,
            &analyzer_config,
        );

        // Sub-packages should inherit the same tag prefix
        assert_eq!(resolved[0].tag_prefix, expected_tag_prefix);
    }

    #[test]
    fn sub_packages_normalize_paths_correctly() {
        let resolved_config = Rc::new(make_resolved_config("test-repo"));

        let pkg_config = PackageConfigBuilder::default()
            .name("parent-pkg")
            .path("workspace")
            .sub_packages(vec![SubPackage {
                name: "sub-pkg".to_string(),
                path: "packages/sub".to_string(),
                ..Default::default()
            }])
            .build()
            .unwrap();

        let workspace_root = Path::new("workspace");
        let expected_tag_prefix = "v";
        let auto_start = false;
        let prerelease = None;
        let analyzer_config = AnalyzerConfig::default();

        let resolved = resolve_sub_packages_full(
            resolved_config,
            pkg_config,
            workspace_root,
            expected_tag_prefix,
            prerelease,
            auto_start,
            &analyzer_config,
        );

        assert_eq!(resolved.len(), 1);

        // Path should contain the sub-package directory
        let sub_path_str = resolved[0]
            .normalized_full_path
            .to_string_lossy()
            .to_string();

        assert!(
            sub_path_str.contains("packages") && sub_path_str.contains("sub")
        );
        // Workspace root should match parent's workspace root
        assert_eq!(resolved[0].normalized_workspace_root, workspace_root);
    }

    #[test]
    fn handles_empty_sub_packages_list() {
        let resolved_config = Rc::new(make_resolved_config("test-repo"));

        let pkg_config = PackageConfigBuilder::default()
            .name("parent-pkg")
            .path(".")
            .build()
            .unwrap();

        let workspace_root = Path::new(".");
        let expected_tag_prefix = "v";
        let auto_start = false;
        let prerelease = None;
        let analyzer_config = AnalyzerConfig::default();

        let resolved = resolve_sub_packages_full(
            resolved_config,
            pkg_config,
            workspace_root,
            expected_tag_prefix,
            prerelease,
            auto_start,
            &analyzer_config,
        );

        // Should have no sub-packages
        assert_eq!(resolved.len(), 0);
    }
}
