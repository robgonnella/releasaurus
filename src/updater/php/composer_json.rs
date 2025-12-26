use log::*;
use serde_json::{Value, json};

use crate::{
    Result,
    forge::request::{FileChange, FileUpdateType},
    updater::{manager::UpdaterPackage, traits::PackageUpdater},
};

/// Handles composer.json file parsing and version updates for PHP packages.
pub struct ComposerJson {}

impl ComposerJson {
    /// Create ComposerJson handler for composer.json version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Load and parse composer.json file from repository into serde_json Value.
    fn load_doc(&self, content: &str) -> Result<Option<Value>> {
        let doc: Value = serde_json::from_str(content)?;
        Ok(Some(doc))
    }
}

impl PackageUpdater for ComposerJson {
    /// Process composer.json files for all PHP packages.
    fn update(
        &self,
        package: &UpdaterPackage,
        _workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.basename != "composer.json" {
                continue;
            }

            if let Some(mut doc) = self.load_doc(&manifest.content)? {
                info!("found composer.json for package: {}", manifest.path);

                // Update the version field
                if let Some(obj) = doc.as_object_mut() {
                    info!(
                        "updating {} version to {}",
                        manifest.path, package.next_version.semver
                    );

                    obj.insert(
                        "version".to_string(),
                        json!(package.next_version.semver.to_string()),
                    );

                    let formatted = serde_json::to_string_pretty(&doc)?;

                    file_changes.push(FileChange {
                        path: manifest.path.clone(),
                        content: formatted,
                        update_type: FileUpdateType::Replace,
                    });
                } else {
                    warn!(
                        "composer.json is not a valid JSON object: {}",
                        manifest.path
                    );
                }
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
    use super::*;
    use crate::{
        analyzer::release::Tag,
        config::release_type::ReleaseType,
        updater::manager::{ManifestFile, UpdaterPackage},
    };

    #[test]
    fn updates_version_field() {
        let composer_json = ComposerJson::new();
        let content = r#"{"name":"vendor/package","version":"1.0.0"}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "composer.json".to_string(),
            basename: "composer.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "vendor/package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Php,
        };

        let result = composer_json.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
    }

    #[test]
    fn inserts_version_field_when_missing() {
        let composer_json = ComposerJson::new();
        let content =
            r#"{"name":"vendor/package","description":"A test package"}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "composer.json".to_string(),
            basename: "composer.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "vendor/package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Php,
        };

        let result = composer_json.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
        assert!(updated.contains("\"description\": \"A test package\""));
    }

    #[test]
    fn preserves_other_fields() {
        let composer_json = ComposerJson::new();
        let content = r#"{
  "name": "vendor/package",
  "version": "1.0.0",
  "description": "A test package",
  "type": "library",
  "require": {
    "php": "^8.0"
  }
}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "composer.json".to_string(),
            basename: "composer.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "vendor/package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Php,
        };

        let result = composer_json.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
        assert!(updated.contains("\"name\": \"vendor/package\""));
        assert!(updated.contains("\"description\": \"A test package\""));
        assert!(updated.contains("\"type\": \"library\""));
        assert!(updated.contains("\"php\": \"^8.0\""));
    }

    #[test]
    fn process_package_handles_multiple_composer_files() {
        let composer_json = ComposerJson::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            path: "packages/a/composer.json".to_string(),
            basename: "composer.json".to_string(),
            content: r#"{"name":"vendor/package-a","version":"1.0.0"}"#
                .to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            path: "packages/b/composer.json".to_string(),
            basename: "composer.json".to_string(),
            content: r#"{"name":"vendor/package-b","version":"1.0.0"}"#
                .to_string(),
        };
        let package = UpdaterPackage {
            package_name: "vendor/package".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Php,
        };

        let result = composer_json.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[test]
    fn process_package_returns_none_when_no_manifest_files() {
        let composer_json = ComposerJson::new();
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Php,
        };

        let result = composer_json.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn process_package_returns_none_when_no_composer_json_files() {
        let composer_json = ComposerJson::new();
        let manifest = ManifestFile {
            is_workspace: false,
            path: "package.json".to_string(),
            basename: "package.json".to_string(),
            content: r#"{"name":"my-package","version":"1.0.0"}"#.to_string(),
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
            release_type: ReleaseType::Php,
        };

        let result = composer_json.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }
}
