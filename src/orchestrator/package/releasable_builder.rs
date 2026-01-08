//! Builder trait for constructing releasable package types.

use crate::{
    ResolvedPackage,
    analyzer::release::Release,
    orchestrator::package::releasable::{
        ReleasablePackage, ReleasableSubPackage, SerializableReleasablePackage,
    },
    updater::manager::{AdditionalManifestFile, ManifestFile},
};

/// Trait for building releasable package types from analyzed data.
/// Enables generic construction of different package representations.
pub trait ReleasablePackageBuilder: Sized {
    fn build(
        name: String,
        release: Release,
        pkg_config: &ResolvedPackage,
        manifest_files: Option<Vec<ManifestFile>>,
        additional_manifest_files: Option<Vec<AdditionalManifestFile>>,
        sub_packages: Vec<ReleasableSubPackage>,
    ) -> Self;
}

impl ReleasablePackageBuilder for ReleasablePackage {
    fn build(
        name: String,
        release: Release,
        pkg_config: &ResolvedPackage,
        manifest_files: Option<Vec<ManifestFile>>,
        additional_manifest_files: Option<Vec<AdditionalManifestFile>>,
        sub_packages: Vec<ReleasableSubPackage>,
    ) -> Self {
        Self {
            name,
            release_type: pkg_config.release_type,
            tag: release.tag,
            notes: release.notes,
            sub_packages,
            additional_manifest_files,
            manifest_files,
        }
    }
}

impl ReleasablePackageBuilder for SerializableReleasablePackage {
    fn build(
        name: String,
        release: Release,
        pkg_config: &ResolvedPackage,
        manifest_files: Option<Vec<ManifestFile>>,
        additional_manifest_files: Option<Vec<AdditionalManifestFile>>,
        sub_packages: Vec<ReleasableSubPackage>,
    ) -> Self {
        Self {
            name,
            path: pkg_config.normalized_full_path.clone(),
            release_type: pkg_config.release_type,
            release,
            sub_packages,
            additional_manifest_files,
            manifest_files,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::release::{Release, Tag},
        config::release_type::ReleaseType,
    };
    use std::path::PathBuf;

    fn create_test_resolved_package() -> ResolvedPackage {
        ResolvedPackage {
            name: "test-package".to_string(),
            normalized_full_path: PathBuf::from("/test/path"),
            normalized_workspace_root: PathBuf::from("/test"),
            release_type: ReleaseType::Node,
            tag_prefix: "v".to_string(),
            sub_packages: vec![],
            prerelease: None,
            auto_start_next: false,
            normalized_additional_paths: vec![],
            compiled_additional_manifests: vec![],
            analyzer_config: Default::default(),
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
            sha: "abc123".to_string(),
            commits: vec![],
            include_author: false,
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
            None,
            None,
            vec![],
        );

        assert_eq!(package.name, "test-package");
        assert_eq!(package.release_type, ReleaseType::Node);
        assert_eq!(package.tag.name, "v1.0.0");
        assert_eq!(package.notes, "Test release notes");
        assert!(package.manifest_files.is_none());
        assert!(package.additional_manifest_files.is_none());
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
            None,
            None,
            vec![],
        );

        assert_eq!(package.name, "test-package");
        assert_eq!(package.path, PathBuf::from("/test/path"));
        assert_eq!(package.release_type, ReleaseType::Node);
        assert_eq!(package.release.tag.name, "v1.0.0");
        assert!(package.manifest_files.is_none());
        assert!(package.additional_manifest_files.is_none());
        assert!(package.sub_packages.is_empty());
    }

    #[test]
    fn test_builder_with_manifest_files() {
        let pkg_config = create_test_resolved_package();
        let release = create_test_release();

        let manifest_files = vec![ManifestFile {
            path: PathBuf::from("/test/package.json"),
            basename: "package.json".to_string(),
            content: "{}".to_string(),
        }];

        let package = ReleasablePackage::build(
            "test-package".to_string(),
            release,
            &pkg_config,
            Some(manifest_files),
            None,
            vec![],
        );

        let manifest_files = package.manifest_files.as_ref().unwrap();
        assert_eq!(manifest_files.len(), 1);
    }

    #[test]
    fn test_builder_with_sub_packages() {
        let pkg_config = create_test_resolved_package();
        let release = create_test_release();

        let sub_packages = vec![ReleasableSubPackage {
            name: "sub-pkg".to_string(),
            release_type: ReleaseType::Node,
            manifest_files: None,
        }];

        let package = ReleasablePackage::build(
            "test-package".to_string(),
            release,
            &pkg_config,
            None,
            None,
            sub_packages,
        );

        assert_eq!(package.sub_packages.len(), 1);
        assert_eq!(package.sub_packages[0].name, "sub-pkg");
    }
}
