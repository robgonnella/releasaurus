use serde_json::{Value, json};

use crate::{
    forge::request::{FileChange, FileUpdateType},
    packages::manifests::ManifestFile,
    result::Result,
    updater::traits::FileUpdater,
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

impl Default for ComposerJson {
    fn default() -> Self {
        ComposerJson::new()
    }
}

impl FileUpdater for ComposerJson {
    /// Process composer.json files for all PHP packages.
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename != "composer.json" {
            return Ok(None);
        }

        let Some(owner) = manifest.owner.as_ref() else {
            return Ok(None);
        };

        if let Some(mut doc) = self.load_doc(&manifest.content)? {
            log::info!(
                "found composer.json for package: {}",
                manifest.path.to_string_lossy()
            );

            // Update the version field
            if let Some(obj) = doc.as_object_mut() {
                log::info!(
                    "updating {} version to {}",
                    manifest.path.to_string_lossy(),
                    owner.tag.semver
                );

                obj.insert(
                    "version".to_string(),
                    json!(owner.tag.semver.to_string()),
                );

                let formatted = serde_json::to_string_pretty(&doc)?;

                return Ok(Some(FileChange {
                    path: manifest.path.to_string_lossy().to_string(),
                    content: formatted,
                    update_type: FileUpdateType::Replace,
                }));
            } else {
                log::warn!(
                    "composer.json is not a valid JSON object: {}",
                    manifest.path.to_string_lossy()
                );
            }
        }

        Ok(None)
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
    fn updates_version_field() {
        let composer_json = ComposerJson::new();
        let content = r#"{"name":"vendor/package","version":"1.0.0"}"#;

        let manifest = ManifestFile {
            path: Path::new("composer.json").to_path_buf(),
            basename: "composer.json".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Php,
            owner: Some(ManifestPackage {
                name: "vendor/package".to_string(),
                release_type: ReleaseType::Php,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = composer_json.update(&manifest).unwrap().unwrap();
        assert!(result.content.contains("\"version\": \"2.0.0\""));
    }

    #[test]
    fn inserts_version_field_when_missing() {
        let composer_json = ComposerJson::new();
        let content =
            r#"{"name":"vendor/package","description":"A test package"}"#;

        let manifest = ManifestFile {
            path: Path::new("composer.json").to_path_buf(),
            basename: "composer.json".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Php,
            owner: Some(ManifestPackage {
                name: "vendor/package".to_string(),
                release_type: ReleaseType::Php,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = composer_json.update(&manifest).unwrap().unwrap();
        assert!(result.content.contains("\"version\": \"2.0.0\""));
        assert!(
            result
                .content
                .contains("\"description\": \"A test package\"")
        );
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
            path: Path::new("composer.json").to_path_buf(),
            basename: "composer.json".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Php,
            owner: Some(ManifestPackage {
                name: "vendor/package".to_string(),
                release_type: ReleaseType::Php,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = composer_json.update(&manifest).unwrap().unwrap();
        assert!(result.content.contains("\"version\": \"2.0.0\""));
        assert!(result.content.contains("\"name\": \"vendor/package\""));
        assert!(
            result
                .content
                .contains("\"description\": \"A test package\"")
        );
        assert!(result.content.contains("\"type\": \"library\""));
        assert!(result.content.contains("\"php\": \"^8.0\""));
    }

    #[test]
    fn process_package_returns_none_when_no_composer_json_files() {
        let composer_json = ComposerJson::new();

        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".into(),
            content: r#"{"name":"my-package","version":"1.0.0"}"#.into(),
            release_type: ReleaseType::Php,
            owner: Some(ManifestPackage {
                name: "vendor/package".to_string(),
                release_type: ReleaseType::Php,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = composer_json.update(&manifest).unwrap();
        assert!(result.is_none());
    }
}
