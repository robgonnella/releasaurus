use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        generic::updater::{GENERIC_VERSION_REGEX, GenericUpdater},
        manager::UpdaterPackage,
        traits::PackageUpdater,
    },
};

/// Handles Gradle build.gradle and build.gradle.kts file parsing and version updates for Java packages.
pub struct Gradle {}

impl Gradle {
    /// Create Gradle handler for build file version updates.
    pub fn new() -> Self {
        Self {}
    }
}

impl PackageUpdater for Gradle {
    fn update(
        &self,
        package: &UpdaterPackage,
        _workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if (manifest.basename == "build.gradle"
                || manifest.basename == "build.gradle.kts")
                && let Some(change) = GenericUpdater::update_manifest(
                    manifest,
                    &package.next_version.semver,
                    &GENERIC_VERSION_REGEX,
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
    fn updates_groovy_version_with_double_quotes() {
        let gradle = Gradle::new();
        let content = r#"version = "1.0.0""#;
        let manifest = ManifestFile {
            path: "build.gradle".to_string(),
            basename: "build.gradle".to_string(),
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

        let result = gradle.update(&package, &[]).unwrap();

        let change = result.unwrap();
        assert_eq!(change.len(), 1);
        assert_eq!(change[0].content, r#"version = "2.0.0""#);
    }

    #[test]
    fn updates_groovy_version_with_single_quotes() {
        let gradle = Gradle::new();
        let content = "version = '1.0.0'";
        let manifest = ManifestFile {
            path: "build.gradle".to_string(),
            basename: "build.gradle".to_string(),
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

        let result = gradle.update(&package, &[]).unwrap();

        let change = result.unwrap();
        assert_eq!(change.len(), 1);
        assert_eq!(change[0].content, "version = '2.0.0'");
    }

    #[test]
    fn updates_kotlin_version() {
        let gradle = Gradle::new();
        let content = r#"version = "1.0.0""#;
        let manifest = ManifestFile {
            path: "build.gradle.kts".to_string(),
            basename: "build.gradle.kts".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v3.5.0".into(),
                semver: semver::Version::parse("3.5.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Java)),
        };

        let result = gradle.update(&package, &[]).unwrap();

        let change = result.unwrap();
        assert_eq!(change.len(), 1);
        assert_eq!(change[0].content, r#"version = "3.5.0""#);
    }

    #[test]
    fn updates_project_version_declaration() {
        let gradle = Gradle::new();
        let content = r#"project.version = "1.0.0""#;
        let manifest = ManifestFile {
            path: "build.gradle".to_string(),
            basename: "build.gradle".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v4.0.0".into(),
                semver: semver::Version::parse("4.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Java)),
        };

        let result = gradle.update(&package, &[]).unwrap();

        let change = result.unwrap();
        assert_eq!(change.len(), 1);
        assert_eq!(change[0].content, r#"project.version = "4.0.0""#);
    }

    #[test]
    fn returns_none_when_no_version_found() {
        let gradle = Gradle::new();
        let content = "dependencies { implementation 'com.example:lib:1.0.0' }";
        let manifest = ManifestFile {
            path: "build.gradle".to_string(),
            basename: "build.gradle".to_string(),
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

        let result = gradle.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn update_handles_multiple_manifests() {
        let gradle = Gradle::new();
        let groovy_manifest = ManifestFile {
            path: "build.gradle".to_string(),
            basename: "build.gradle".to_string(),
            content: r#"version = "1.0.0""#.to_string(),
        };
        let kotlin_manifest = ManifestFile {
            path: "build.gradle.kts".to_string(),
            basename: "build.gradle.kts".to_string(),
            content: r#"version = "1.0.0""#.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![groovy_manifest, kotlin_manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Java)),
        };

        let result = gradle.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[test]
    fn update_returns_none_when_no_changes() {
        let gradle = Gradle::new();
        let manifest = ManifestFile {
            path: "pom.xml".to_string(),
            basename: "pom.xml".to_string(),
            content: "<version>1.0.0</version>".to_string(),
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

        let result = gradle.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn preserves_whitespace_formatting() {
        let gradle = Gradle::new();
        let content = "version   =   \"1.0.0\"";
        let manifest = ManifestFile {
            path: "build.gradle".to_string(),
            basename: "build.gradle".to_string(),
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

        let result = gradle.update(&package, &[]).unwrap();

        let change = result.unwrap();
        assert_eq!(change.len(), 1);
        assert_eq!(change[0].content, "version   =   \"2.0.0\"");
    }
}
