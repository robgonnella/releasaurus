use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        manager::UpdaterPackage,
        php::{composer_json::ComposerJson, composer_lock::ComposerLock},
        traits::PackageUpdater,
    },
};

/// PHP package updater for Composer projects.
pub struct PhpUpdater {
    composer_json: ComposerJson,
    composer_lock: ComposerLock,
}

impl PhpUpdater {
    /// Create PHP updater for Composer composer.json files.
    pub fn new() -> Self {
        Self {
            composer_json: ComposerJson::new(),
            composer_lock: ComposerLock::new(),
        }
    }
}

impl PackageUpdater for PhpUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];
        let mut new_composer_json = None;
        let mut composer_lock_manifest = None;

        for manifest in package.manifest_files.iter() {
            if manifest.basename == "composer.json"
                && let Some(changes) =
                    self.composer_json.update(package, workspace_packages)?
            {
                new_composer_json = Some(changes[0].content.clone());
                file_changes.extend(changes);
            }

            if manifest.basename == "composer.lock" {
                composer_lock_manifest = Some(manifest);
            }
        }

        if let Some(lock_manifest) = composer_lock_manifest
            && let Some(new_json) = new_composer_json
            && let Some(change) = self
                .composer_lock
                .get_lock_change(lock_manifest, &new_json)?
        {
            file_changes.push(change);
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, rc::Rc};

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
    fn processes_php_project() {
        let updater = PhpUpdater::new();
        let content = r#"{"name":"vendor/package","version":"1.0.0"}"#;
        let manifest = ManifestFile {
            path: Path::new("composer.json").to_path_buf(),
            basename: "composer.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "vendor/package".to_string(),
            manifest_files: vec![manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Php)),
        };

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_php_files() {
        let updater = PhpUpdater::new();
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
            updater: Rc::new(Updater::new(ReleaseType::Php)),
        };

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn updates_both_composer_json_and_lock() {
        let updater = PhpUpdater::new();
        let json_manifest = ManifestFile {
            path: Path::new("composer.json").to_path_buf(),
            basename: "composer.json".to_string(),
            content: r#"{"name":"vendor/package","version":"1.0.0"}"#
                .to_string(),
        };
        let lock_manifest = ManifestFile {
            path: Path::new("composer.lock").to_path_buf(),
            basename: "composer.lock".to_string(),
            content: r#"{"content-hash":"old","packages":[]}"#.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "vendor/package".to_string(),
            manifest_files: vec![json_manifest, lock_manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Php)),
        };

        let result = updater.update(&package, &[]).unwrap().unwrap();

        assert_eq!(result.len(), 2);
        let json_change =
            result.iter().find(|c| c.path == "composer.json").unwrap();
        let lock_change =
            result.iter().find(|c| c.path == "composer.lock").unwrap();
        assert!(json_change.content.contains("2.0.0"));
        assert!(lock_change.content.contains("content-hash"));
        assert!(!lock_change.content.contains("\"old\""));
    }

    #[test]
    fn skips_lock_update_when_no_composer_json() {
        let updater = PhpUpdater::new();
        let lock_manifest = ManifestFile {
            path: Path::new("composer.lock").to_path_buf(),
            basename: "composer.lock".to_string(),
            content: r#"{"content-hash":"old","packages":[]}"#.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "vendor/package".to_string(),
            manifest_files: vec![lock_manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Php)),
        };

        let result = updater.update(&package, &[]).unwrap();

        // No composer.json means no updates (lock depends on json)
        assert!(result.is_none());
    }
}
