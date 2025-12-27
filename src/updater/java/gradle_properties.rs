use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        generic::updater::GenericUpdater, manager::UpdaterPackage,
        traits::PackageUpdater,
    },
};

/// Handles gradle.properties file parsing and version updates for Java packages.
pub struct GradleProperties {}

impl GradleProperties {
    /// Create GradleProperties handler for gradle.properties version updates.
    pub fn new() -> Self {
        Self {}
    }
}

impl PackageUpdater for GradleProperties {
    /// Update version fields in gradle.properties files for all Java packages.
    fn update(
        &self,
        package: &UpdaterPackage,
        _workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.basename == "gradle.properties"
                && let Some(change) = GenericUpdater::update_manifest(
                    manifest,
                    &package.next_version.semver,
                )
            {
                file_changes.push(change);
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::{
        analyzer::release::Tag,
        config::release_type::ReleaseType,
        updater::{
            dispatch::Updater,
            manager::{ManifestFile, UpdaterPackage},
        },
    };

    #[test]
    fn updates_version_property() {
        let properties = GradleProperties::new();
        let content = "version=1.0.0";
        let manifest = ManifestFile {
            is_workspace: false,
            path: "gradle.properties".to_string(),
            basename: "gradle.properties".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Java)),
        };

        let result = properties.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].content, "version=2.0.0");
    }

    #[test]
    fn preserves_whitespace_around_equals() {
        let properties = GradleProperties::new();
        let content = "version  =  1.0.0";
        let manifest = ManifestFile {
            is_workspace: false,
            path: "gradle.properties".to_string(),
            basename: "gradle.properties".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::parse("3.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Java)),
        };

        let result = properties.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].content, "version  =  3.0.0");
    }

    #[test]
    fn preserves_leading_whitespace() {
        let properties = GradleProperties::new();
        let content = "  version=1.0.0";
        let manifest = ManifestFile {
            is_workspace: false,
            path: "gradle.properties".to_string(),
            basename: "gradle.properties".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.5.0".into(),
                semver: semver::Version::parse("2.5.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Java)),
        };

        let result = properties.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].content, "  version=2.5.0");
    }

    #[test]
    fn preserves_other_properties() {
        let properties = GradleProperties::new();
        let content =
            "org.gradle.jvmargs=-Xmx2048m\nversion=1.0.0\ngroup=com.example";
        let manifest = ManifestFile {
            is_workspace: false,
            path: "gradle.properties".to_string(),
            basename: "gradle.properties".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Java)),
        };

        let result = properties.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        let updated = changes[0].content.clone();

        assert!(updated.contains("org.gradle.jvmargs=-Xmx2048m"));
        assert!(updated.contains("version=2.0.0"));
        assert!(updated.contains("group=com.example"));
    }

    #[test]
    fn returns_none_when_no_version_property() {
        let properties = GradleProperties::new();
        let content = "org.gradle.jvmargs=-Xmx2048m\ngroup=com.example";
        let manifest = ManifestFile {
            is_workspace: false,
            path: "gradle.properties".to_string(),
            basename: "gradle.properties".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Java)),
        };

        let result = properties.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn process_package_handles_multiple_properties_files() {
        let properties = GradleProperties::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            path: "module1/gradle.properties".to_string(),
            basename: "gradle.properties".to_string(),
            content: "version=1.0.0".to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            path: "module2/gradle.properties".to_string(),
            basename: "gradle.properties".to_string(),
            content: "version=1.0.0".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Java)),
        };

        let result = properties.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[test]
    fn process_package_returns_none_when_no_gradle_properties() {
        let properties = GradleProperties::new();
        let manifest = ManifestFile {
            is_workspace: false,
            path: "build.gradle".to_string(),
            basename: "build.gradle".to_string(),
            content: "version = \"1.0.0\"".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Java)),
        };

        let result = properties.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn updates_commented_version_lines() {
        let properties = GradleProperties::new();
        let content = "# version=0.0.1\nversion=1.0.0";
        let manifest = ManifestFile {
            is_workspace: false,
            path: "gradle.properties".to_string(),
            basename: "gradle.properties".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Java)),
        };

        let result = properties.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);

        let updated = changes[0].content.clone();
        assert!(updated.contains("# version=2.0.0"));
        assert!(updated.contains("version=2.0.0"));
    }
}
