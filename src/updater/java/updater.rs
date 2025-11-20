use async_trait::async_trait;

use crate::{
    forge::request::FileChange,
    result::Result,
    updater::{framework::UpdaterPackage, traits::PackageUpdater},
};

use super::gradle::Gradle;
use super::gradle_properties::GradleProperties;
use super::maven::Maven;

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

#[async_trait]
impl PackageUpdater for JavaUpdater {
    async fn update(
        &self,
        package: &UpdaterPackage,
        // workspaces not supported for java projects
        _workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        // Try Maven first (pom.xml) - takes precedence
        if let Some(changes) = self.maven.process_package(package).await? {
            file_changes.extend(changes);
        }

        // Try Gradle build files (build.gradle, build.gradle.kts)
        if let Some(changes) = self.gradle.process_package(package).await? {
            file_changes.extend(changes);
        }

        // Try gradle.properties
        if let Some(changes) =
            self.gradle_properties.process_package(package).await?
        {
            file_changes.extend(changes);
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}
