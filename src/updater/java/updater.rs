use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        composite::CompositeUpdater,
        java::{
            gradle::Gradle, gradle_properties::GradleProperties, maven::Maven,
        },
        manager::UpdaterPackage,
        traits::PackageUpdater,
    },
};

/// Java package updater supporting Maven and Gradle projects.
pub struct JavaUpdater {
    composite: CompositeUpdater,
}

impl JavaUpdater {
    /// Create Java updater for Maven pom.xml and Gradle build files.
    pub fn new() -> Self {
        Self {
            composite: CompositeUpdater::new(vec![
                Box::new(Gradle::new()),
                Box::new(GradleProperties::new()),
                Box::new(Maven::new()),
            ]),
        }
    }
}

impl PackageUpdater for JavaUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        self.composite.update(package, workspace_packages)
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
    fn processes_maven_project() {
        let updater = JavaUpdater::new();
        let content = r#"<?xml version="1.0"?>
<project>
    <version>1.0.0</version>
</project>"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "pom.xml".to_string(),
            basename: "pom.xml".to_string(),
            content: content.to_string(),
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

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_java_files() {
        let updater = JavaUpdater::new();
        let manifest = ManifestFile {
            is_workspace: false,
            path: "package.json".to_string(),
            basename: "package.json".to_string(),
            content: r#"{"version":"1.0.0"}"#.to_string(),
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

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }
}
