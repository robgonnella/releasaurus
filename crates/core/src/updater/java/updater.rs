use crate::{
    config::release_type::ReleaseType,
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{
        composite::CompositeUpdater,
        java::{
            gradle::Gradle, gradle_properties::GradleProperties,
            libs_versions_toml::LibsVersionsToml, maven::Maven,
        },
        traits::FileUpdater,
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

impl FileUpdater for JavaUpdater {
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if !matches!(manifest.release_type, ReleaseType::Java) {
            return Ok(None);
        }
        self.composite.update(manifest)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        config::release_type::ReleaseType,
        forge::request::Tag,
        packages::manifests::{ManifestFile, ManifestPackage},
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
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".to_string(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = updater.update(&manifest).unwrap();
        assert!(result.unwrap().content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_java_files() {
        let updater = JavaUpdater::new();

        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".to_string(),
            content: r#"{"version":"1.0.0"}"#.to_string(),
            release_type: ReleaseType::Java,
            owner: Some(ManifestPackage {
                name: "test".to_string(),
                release_type: ReleaseType::Java,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = updater.update(&manifest).unwrap();
        assert!(result.is_none());
    }
}
