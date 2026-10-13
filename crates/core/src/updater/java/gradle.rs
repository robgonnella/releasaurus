use regex::Regex;
use std::sync::LazyLock;

use crate::{
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{generic::updater::GenericUpdater, traits::FileUpdater},
};

/// Gradle-specific version regex that only matches the project `version`
/// property. It anchors to the start of the line and only allows an
/// optional `project.` prefix, preventing false matches on variables like
/// `awsSoftwareVersion`, `kotlinVersion`, etc. Whatever string literal is
/// assigned is replaced; an expression (`version = rootProject.version`)
/// has nothing to match.
static GRADLE_VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?mi)^\s*(?:project\.)?version\s*=\s*['"](?<version>[^'"]*)['"]"#,
    )
    .unwrap()
});

/// Handles Gradle build.gradle and build.gradle.kts file parsing and version updates for Java packages.
pub struct Gradle {}

impl Gradle {
    /// Create Gradle handler for build file version updates.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Gradle {
    fn default() -> Self {
        Gradle::new()
    }
}

impl FileUpdater for Gradle {
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename == "build.gradle"
            || manifest.basename == "build.gradle.kts"
        {
            return Ok(GenericUpdater::update_manifest(
                manifest,
                &GRADLE_VERSION_REGEX,
            ));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use std::path::Path;

    use semver::Version;

    use crate::{
        config::release_type::ReleaseType,
        forge::request::{Tag, TagVersion},
        packages::manifests::{ManifestFile, ManifestPackage},
    };

    use super::*;

    #[test]
    fn updates_groovy_version_with_double_quotes() {
        let gradle = Gradle::new();
        let content = r#"version = "1.0.0""#;

        let manifest = ManifestFile {
            path: Path::new("build.gradle").to_path_buf(),
            basename: "build.gradle".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    version: TagVersion::Semantic(Version::new(2, 0, 0)),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = gradle.update(&manifest).unwrap();
        let change = result.unwrap();
        assert_eq!(change.content, r#"version = "2.0.0""#);
    }

    #[test]
    fn updates_groovy_version_with_single_quotes() {
        let gradle = Gradle::new();
        let content = "version = '1.0.0'";

        let manifest = ManifestFile {
            path: Path::new("build.gradle").to_path_buf(),
            basename: "build.gradle".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    version: TagVersion::Semantic(Version::new(2, 0, 0)),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = gradle.update(&manifest).unwrap();
        let change = result.unwrap();
        assert_eq!(change.content, "version = '2.0.0'");
    }

    #[test]
    fn updates_kotlin_version() {
        let gradle = Gradle::new();
        let content = r#"version = "1.0.0""#;

        let manifest = ManifestFile {
            path: Path::new("build.gradle.kts").to_path_buf(),
            basename: "build.gradle.kts".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v3.5.0".into(),
                    version: TagVersion::Semantic(Version::new(3, 5, 0)),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = gradle.update(&manifest).unwrap();
        let change = result.unwrap();
        assert_eq!(change.content, r#"version = "3.5.0""#);
    }

    #[test]
    fn updates_project_version_declaration() {
        let gradle = Gradle::new();
        let content = r#"project.version = "1.0.0""#;

        let manifest = ManifestFile {
            path: Path::new("build.gradle").to_path_buf(),
            basename: "build.gradle".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v4.0.0".into(),
                    version: TagVersion::Semantic(Version::new(4, 0, 0)),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = gradle.update(&manifest).unwrap();
        let change = result.unwrap();
        assert_eq!(change.content, r#"project.version = "4.0.0""#);
    }

    #[test]
    fn returns_none_when_no_version_found() {
        let gradle = Gradle::new();
        let content = "dependencies { implementation 'com.example:lib:1.0.0' }";

        let manifest = ManifestFile {
            path: Path::new("build.gradle").to_path_buf(),
            basename: "build.gradle".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    version: TagVersion::Semantic(Version::new(2, 0, 0)),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = gradle.update(&manifest).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn update_returns_none_when_no_changes() {
        let gradle = Gradle::new();

        let manifest = ManifestFile {
            path: Path::new("pom.xml").to_path_buf(),
            basename: "pom.xml".to_string(),
            content: "<version>1.0.0</version>".to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    version: TagVersion::Semantic(Version::new(2, 0, 0)),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = gradle.update(&manifest).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn does_not_update_ext_variables_containing_version() {
        let gradle = Gradle::new();
        let content = r#"
buildscript {
    ext {
        awsSoftwareVersion = "1.0.0"
        kotlinVersion = "1.9.20"
        springBootVersion = "3.2.0"
    }
}

version = "1.0.0"
"#
        .trim();

        let manifest = ManifestFile {
            path: Path::new("build.gradle").to_path_buf(),
            basename: "build.gradle".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    version: TagVersion::Semantic(Version::new(2, 0, 0)),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = gradle.update(&manifest).unwrap();
        let change = result.unwrap();
        assert!(
            change.content.contains(r#"awsSoftwareVersion = "1.0.0""#),
            "awsSoftwareVersion should not be updated"
        );
        assert!(
            change.content.contains(r#"kotlinVersion = "1.9.20""#),
            "kotlinVersion should not be updated"
        );
        assert!(
            change.content.contains(r#"springBootVersion = "3.2.0""#),
            "springBootVersion should not be updated"
        );
        assert!(
            change.content.contains(r#"version = "2.0.0""#),
            "project version should be updated"
        );
    }

    #[test]
    fn preserves_whitespace_formatting() {
        let gradle = Gradle::new();
        let content = "version   =   \"1.0.0\"";

        let manifest = ManifestFile {
            path: Path::new("build.gradle").to_path_buf(),
            basename: "build.gradle".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".into(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    version: TagVersion::Semantic(Version::new(2, 0, 0)),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = gradle.update(&manifest).unwrap();
        let change = result.unwrap();
        assert_eq!(change.content, "version   =   \"2.0.0\"");
    }

    /// Whatever value is set is replaced, whatever its shape.
    #[test]
    fn updates_a_calver_version() {
        let updater = Gradle::new();
        let manifest = ManifestFile {
            path: Path::new("build.gradle").to_path_buf(),
            basename: "build.gradle".to_string(),
            content: "version = \"26.09\"\n".to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "pkg".to_string(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v26.10".into(),
                    version: TagVersion::Strftime(
                        Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap(),
                        "%y.%m".into(),
                    ),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let updated = updater.update(&manifest).unwrap().unwrap().content;

        assert_eq!(updated, "version = \"26.10\"\n");
    }

    /// An expression is code, not a value of ours to write.
    #[test]
    fn leaves_a_version_expression_alone() {
        let manifest = ManifestFile {
            path: Path::new("build.gradle").to_path_buf(),
            basename: "build.gradle".to_string(),
            content: "version = rootProject.version\n".to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "pkg".to_string(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    version: TagVersion::Semantic(Version::new(2, 0, 0)),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        assert!(Gradle::new().update(&manifest).unwrap().is_none());
    }
}
