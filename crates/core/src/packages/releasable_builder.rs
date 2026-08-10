//! Builder trait for constructing releasable package types.

use regex::Regex;

use crate::{
    analyzer::release::Release,
    packages::{
        releasable::{
            ReleasablePackage, ReleasableSubPackage,
            SerializableReleasablePackage,
        },
        resolved::ResolvedPackage,
    },
    updater::manager::ManifestTarget,
};

/// Trait for building releasable package types from analyzed data.
/// Enables generic construction of different package representations.
pub trait ReleasablePackageBuilder: Sized {
    fn build(
        name: String,
        release: Release,
        pkg_config: &ResolvedPackage,
        manifest_targets: Vec<ManifestTarget>,
        additional_manifest_targets: Vec<(ManifestTarget, Regex)>,
        sub_packages: Vec<ReleasableSubPackage>,
    ) -> Self;
}

impl ReleasablePackageBuilder for ReleasablePackage {
    fn build(
        name: String,
        release: Release,
        pkg_config: &ResolvedPackage,
        manifest_targets: Vec<ManifestTarget>,
        additional_manifest_targets: Vec<(ManifestTarget, Regex)>,
        sub_packages: Vec<ReleasableSubPackage>,
    ) -> Self {
        Self {
            name,
            path: pkg_config.normalized_full_path.clone(),
            release_type: pkg_config.release_type,
            tag: release.tag,
            notes: release.notes,
            tag_compare_link: release.tag_compare_link,
            sha_compare_link: release.sha_compare_link,
            sub_packages,
            additional_manifest_targets,
            manifest_targets,
        }
    }
}

impl ReleasablePackageBuilder for SerializableReleasablePackage {
    fn build(
        name: String,
        release: Release,
        pkg_config: &ResolvedPackage,
        manifest_targets: Vec<ManifestTarget>,
        additional_manifest_targets: Vec<(ManifestTarget, Regex)>,
        sub_packages: Vec<ReleasableSubPackage>,
    ) -> Self {
        Self {
            name,
            path: pkg_config.normalized_full_path.clone(),
            release_type: pkg_config.release_type,
            release,
            sub_packages,
            additional_manifest_targets,
            manifest_targets,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        config::{
            defaults::DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE,
            release_type::ReleaseType,
        },
        forge::request::Tag,
    };

    use super::*;

    fn create_test_resolved_package() -> ResolvedPackage {
        ResolvedPackage {
            name: "test-package".to_string(),
            normalized_full_path: PathBuf::from("/test/path"),
            normalized_workspace_root: PathBuf::from("/test"),
            release_type: ReleaseType::Node,
            tag_prefix: "v".to_string(),
            sub_packages: vec![],
            aggregate_prereleases: false,
            normalized_additional_paths: vec![],
            additional_manifests: vec![],
            analyzer_config: Default::default(),
            versioning_config: Default::default(),
            commit_message_template: DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE
                .into(),
            pr_title_template: DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE.into(),
        }
    }

    fn create_test_release() -> Release {
        Release {
            tag: Tag {
                sha: "abc123".to_string(),
                name: "v1.0.0".to_string(),
                semver: semver::Version::new(1, 0, 0),
                timestamp: Some(1234567890),
            },
            link: "https://example.com".to_string(),
            tag_compare_link: "https://example.com/compare/v0.9.0...v1.0.0"
                .into(),
            sha_compare_link: "https://example.com/compare/v0.9.0...abc123"
                .into(),
            sha: "abc123".to_string(),
            short_sha: "abc".to_string(),
            commits: vec![],
            include_author: false,
            include_pr_link: false,
            notes: "Test release notes".to_string(),
            timestamp: 1234567890,
        }
    }

    #[test]
    fn test_releasable_package_builder() {
        let pkg_config = create_test_resolved_package();
        let release = create_test_release();

        let package = ReleasablePackage::build(
            "test-package".to_string(),
            release.clone(),
            &pkg_config,
            vec![],
            vec![],
            vec![],
        );

        assert_eq!(package.name, "test-package");
        assert_eq!(package.release_type, ReleaseType::Node);
        assert_eq!(package.tag.name, "v1.0.0");
        assert_eq!(package.notes, "Test release notes");
        assert!(package.manifest_targets.is_empty());
        assert!(package.additional_manifest_targets.is_empty());
        assert!(package.sub_packages.is_empty());
    }

    #[test]
    fn test_serializable_releasable_package_builder() {
        let pkg_config = create_test_resolved_package();
        let release = create_test_release();

        let package = SerializableReleasablePackage::build(
            "test-package".to_string(),
            release.clone(),
            &pkg_config,
            vec![],
            vec![],
            vec![],
        );

        assert_eq!(package.name, "test-package");
        assert_eq!(package.path, PathBuf::from("/test/path"));
        assert_eq!(package.release_type, ReleaseType::Node);
        assert_eq!(package.release.tag.name, "v1.0.0");
        assert!(package.manifest_targets.is_empty());
        assert!(package.additional_manifest_targets.is_empty());
        assert!(package.sub_packages.is_empty());
    }

    #[test]
    fn test_builder_with_manifest_targets() {
        let pkg_config = create_test_resolved_package();
        let release = create_test_release();

        let manifest_targets = vec![ManifestTarget {
            path: PathBuf::from("/test/package.json"),
            basename: "package.json".to_string(),
        }];

        let package = ReleasablePackage::build(
            "test-package".to_string(),
            release,
            &pkg_config,
            manifest_targets,
            vec![],
            vec![],
        );

        assert_eq!(package.manifest_targets.len(), 1);
    }

    #[test]
    fn test_builder_with_sub_packages() {
        let pkg_config = create_test_resolved_package();
        let release = create_test_release();

        let sub_packages = vec![ReleasableSubPackage {
            name: "sub-pkg".to_string(),
            path: PathBuf::from("packages/sub-pkg"),
            release_type: ReleaseType::Node,
            manifest_targets: vec![],
        }];

        let package = ReleasablePackage::build(
            "test-package".to_string(),
            release,
            &pkg_config,
            vec![],
            vec![],
            sub_packages,
        );

        assert_eq!(package.sub_packages.len(), 1);
        assert_eq!(package.sub_packages[0].name, "sub-pkg");
    }
}
