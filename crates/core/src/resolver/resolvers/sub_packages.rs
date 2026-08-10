use std::path::Path;

use crate::{
    analyzer::config::AnalyzerConfig,
    config::{package::PackageConfig, versioning::VersioningConfig},
    packages::resolved::ResolvedPackage,
    resolver::resolvers::{
        package_name::resolve_sub_package_name,
        path_utils::normalize_package_path,
    },
};

/// Resolves all sub-packages for a package.
pub fn resolve_sub_packages_full(
    package_config: PackageConfig,
    repo_name: &str,
    normalized_workspace_root: &Path,
    tag_prefix: &str,
    analyzer_config: &AnalyzerConfig,
    versioning_config: &VersioningConfig,
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
            let name = resolve_sub_package_name(s, &workspace_root, repo_name);

            let sub_path = normalized_workspace_root
                .join(&s.path)
                .to_string_lossy()
                .to_string();

            let normalized_sub_full_path = normalize_package_path(&sub_path);

            ResolvedPackage {
                name,
                normalized_workspace_root: normalized_workspace_root
                    .to_path_buf(),
                normalized_full_path: normalized_sub_full_path,
                release_type: s.release_type.unwrap_or_default(),
                tag_prefix: tag_prefix.to_string(),
                sub_packages: vec![],
                aggregate_prereleases: false,
                normalized_additional_paths: vec![],
                additional_manifests: vec![],
                analyzer_config: analyzer_config.clone(),
                versioning_config: versioning_config.clone(),
                // A sub-package shares its parent's release PR, so it
                // never renders a commit message or title of its own.
                commit_message_template: String::new(),
                pr_title_template: String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::config::package::{PackageConfigBuilder, SubPackage};

    use super::*;

    #[test]
    fn resolves_sub_packages_with_explicit_names() {
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
        let analyzer_config = AnalyzerConfig::default();
        let versioning_config = VersioningConfig::default();

        let resolved = resolve_sub_packages_full(
            pkg_config,
            "test-repo",
            workspace_root,
            tag_prefix,
            &analyzer_config,
            &versioning_config,
        );

        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].name, "sub-pkg-a");
        assert_eq!(resolved[1].name, "sub-pkg-b");
    }

    #[test]
    fn resolves_sub_packages_with_auto_generated_names() {
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
        let analyzer_config = AnalyzerConfig::default();
        let versioning_config = VersioningConfig::default();

        let resolved = resolve_sub_packages_full(
            pkg_config,
            "test-repo",
            workspace_root,
            tag_prefix,
            &analyzer_config,
            &versioning_config,
        );

        assert_eq!(resolved.len(), 1);
        // Name should be derived from the last path component
        assert_eq!(resolved[0].name, "my-package");
    }

    #[test]
    fn sub_packages_inherit_parent_tag_prefix() {
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
        let analyzer_config = AnalyzerConfig::default();
        let versioning_config = VersioningConfig::default();

        let resolved = resolve_sub_packages_full(
            pkg_config,
            "test-repo",
            workspace_root,
            expected_tag_prefix,
            &analyzer_config,
            &versioning_config,
        );

        // Sub-packages should inherit the same tag prefix
        assert_eq!(resolved[0].tag_prefix, expected_tag_prefix);
    }

    #[test]
    fn sub_packages_normalize_paths_correctly() {
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
        let analyzer_config = AnalyzerConfig::default();
        let versioning_config = VersioningConfig::default();

        let resolved = resolve_sub_packages_full(
            pkg_config,
            "test-repo",
            workspace_root,
            expected_tag_prefix,
            &analyzer_config,
            &versioning_config,
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
        let pkg_config = PackageConfigBuilder::default()
            .name("parent-pkg")
            .path(".")
            .build()
            .unwrap();

        let workspace_root = Path::new(".");
        let expected_tag_prefix = "v";
        let analyzer_config = AnalyzerConfig::default();
        let versioning_config = VersioningConfig::default();

        let resolved = resolve_sub_packages_full(
            pkg_config,
            "test-repo",
            workspace_root,
            expected_tag_prefix,
            &analyzer_config,
            &versioning_config,
        );

        // Should have no sub-packages
        assert_eq!(resolved.len(), 0);
    }
}
