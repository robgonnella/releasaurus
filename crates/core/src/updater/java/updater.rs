use crate::{
    forge::request::FileChange,
    result::Result,
    updater::{
        composite::CompositeUpdater,
        java::{
            gradle::Gradle, gradle_properties::GradleProperties,
            libs_versions_toml::LibsVersionsToml, maven::Maven,
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
                Box::new(LibsVersionsToml::new()),
                Box::new(Maven::new()),
            ]),
        }
    }
}

impl Default for JavaUpdater {
    fn default() -> Self {
        JavaUpdater::new()
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
    use std::{path::Path, rc::Rc};

    use crate::{
        config::release_type::ReleaseType, forge::request::Tag,
        packages::manifests::ManifestFile, updater::dispatch::Updater,
    };

    use super::*;

    #[test]
    fn processes_maven_project() {
        let updater = JavaUpdater::new();
        let content = r#"<?xml version="1.0"?>
<project>
    <version>1.0.0</version>
</project>"#;
        let manifest = ManifestFile {
            path: Path::new("pom.xml").to_path_buf(),
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
            path: Path::new("package.json").to_path_buf(),
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
