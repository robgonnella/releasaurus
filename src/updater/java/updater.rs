use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        java::{
            gradle::Gradle, gradle_properties::GradleProperties, maven::Maven,
        },
        manager::UpdaterPackage,
        traits::PackageUpdater,
    },
};

/// Java package updater supporting Maven and Gradle projects.
pub struct JavaUpdater {
    maven: Maven,
    gradle: Gradle,
    gradle_properties: GradleProperties,
}

impl JavaUpdater {
    /// Create Java updater for Maven pom.xml and Gradle build files.
    pub fn new() -> Self {
        Self {
            maven: Maven::new(),
            gradle: Gradle::new(),
            gradle_properties: GradleProperties::new(),
        }
    }
}

impl PackageUpdater for JavaUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        // workspaces not supported for java projects
        _workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        // Try Maven first (pom.xml) - takes precedence
        if let Some(changes) = self.maven.process_package(package)? {
            file_changes.extend(changes);
        }

        // Try Gradle build files (build.gradle, build.gradle.kts)
        if let Some(changes) = self.gradle.process_package(package)? {
            file_changes.extend(changes);
        }

        // Try gradle.properties
        if let Some(changes) =
            self.gradle_properties.process_package(package)?
        {
            file_changes.extend(changes);
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::release::Tag,
        config::release_type::ReleaseType,
        updater::manager::{ManifestFile, UpdaterPackage},
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
            release_type: ReleaseType::Java,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_some());
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
            release_type: ReleaseType::Java,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_none());
    }
}
