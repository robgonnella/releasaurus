use regex::Regex;
use std::sync::LazyLock;

use crate::{
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{generic::updater::GenericUpdater, traits::FileUpdater},
};

/// Gradle properties-specific version regex that only matches a standalone
/// `version` property. Prevents false matches on properties like
/// `awsSoftwareVersion`, `kotlinVersion`, etc.
static GRADLE_PROPERTIES_VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?mi)(?<start>^\s*version\s*=\s*['"]?)(?<version>\d+\.\d+\.\d+-?.*?)(?<end>['",].*)?$"#).unwrap()
});

/// Handles gradle.properties file parsing and version updates for Java packages.
pub struct GradleProperties {}

impl GradleProperties {
    /// Create GradleProperties handler for gradle.properties version updates.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for GradleProperties {
    fn default() -> Self {
        GradleProperties::new()
    }
}

impl FileUpdater for GradleProperties {
    /// Update version fields in gradle.properties files for all Java packages.
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename == "gradle.properties" {
            return Ok(GenericUpdater::update_manifest(
                manifest,
                &GRADLE_PROPERTIES_VERSION_REGEX,
            ));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use semver::Version;

    use crate::{
        config::release_type::ReleaseType,
        forge::request::Tag,
        packages::manifests::{ManifestFile, ManifestPackage},
    };

    use super::*;

    #[test]
    fn updates_version_property() {
        let properties = GradleProperties::new();
        let content = "version=1.0.0";

        let manifest = ManifestFile {
            path: Path::new("gradle.properties").to_path_buf(),
            basename: "gradle.properties".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = properties.update(&manifest).unwrap();
        let changes = result.unwrap();
        assert_eq!(changes.content, "version=2.0.0");
    }

    #[test]
    fn preserves_whitespace_around_equals() {
        let properties = GradleProperties::new();
        let content = "version  =  1.0.0";

        let manifest = ManifestFile {
            path: Path::new("gradle.properties").to_path_buf(),
            basename: "gradle.properties".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v3.0.0".into(),
                    semver: Version::new(3, 0, 0),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = properties.update(&manifest).unwrap();
        let change = result.unwrap();
        assert_eq!(change.content, "version  =  3.0.0");
    }

    #[test]
    fn preserves_leading_whitespace() {
        let properties = GradleProperties::new();
        let content = "  version=1.0.0";

        let manifest = ManifestFile {
            path: Path::new("gradle.properties").to_path_buf(),
            basename: "gradle.properties".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.5.0".into(),
                    semver: Version::new(2, 5, 0),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = properties.update(&manifest).unwrap();
        let change = result.unwrap();
        assert_eq!(change.content, "  version=2.5.0");
    }

    #[test]
    fn preserves_other_properties() {
        let properties = GradleProperties::new();
        let content =
            "org.gradle.jvmargs=-Xmx2048m\nversion=1.0.0\ngroup=com.example";

        let manifest = ManifestFile {
            path: Path::new("gradle.properties").to_path_buf(),
            basename: "gradle.properties".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = properties.update(&manifest).unwrap();

        let change = result.unwrap();
        assert!(change.content.contains("org.gradle.jvmargs=-Xmx2048m"));
        assert!(change.content.contains("version=2.0.0"));
        assert!(change.content.contains("group=com.example"));
    }

    #[test]
    fn returns_none_when_no_version_property() {
        let properties = GradleProperties::new();
        let content = "org.gradle.jvmargs=-Xmx2048m\ngroup=com.example";

        let manifest = ManifestFile {
            path: Path::new("gradle.properties").to_path_buf(),
            basename: "gradle.properties".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = properties.update(&manifest).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn process_package_returns_none_when_no_gradle_properties() {
        let properties = GradleProperties::new();

        let manifest = ManifestFile {
            path: Path::new("build.gradle").to_path_buf(),
            basename: "build.gradle".to_string(),
            content: "version = \"1.0.0\"".to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = properties.update(&manifest).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn does_not_update_properties_containing_version_in_name() {
        let properties = GradleProperties::new();
        let content =
            "awsSoftwareVersion=1.0.0\nkotlinVersion=1.9.20\nversion=1.0.0";

        let manifest = ManifestFile {
            path: Path::new("gradle.properties").to_path_buf(),
            basename: "gradle.properties".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = properties.update(&manifest).unwrap();
        let change = result.unwrap();

        assert!(
            change.content.contains("awsSoftwareVersion=1.0.0"),
            "awsSoftwareVersion should not be updated"
        );
        assert!(
            change.content.contains("kotlinVersion=1.9.20"),
            "kotlinVersion should not be updated"
        );
        assert!(
            change.content.contains("version=2.0.0"),
            "version property should be updated"
        );
    }

    #[test]
    fn does_not_update_commented_version_lines() {
        let properties = GradleProperties::new();
        let content = "# version=0.0.1\nversion=1.0.0";

        let manifest = ManifestFile {
            path: Path::new("gradle.properties").to_path_buf(),
            basename: "gradle.properties".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = properties.update(&manifest).unwrap();
        let change = result.unwrap();

        assert!(
            change.content.contains("# version=0.0.1"),
            "commented version should not be updated"
        );
        assert!(change.content.contains("version=2.0.0"));
    }
}
