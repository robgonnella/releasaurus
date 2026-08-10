use serde_json::{Value, json};

use crate::{
    forge::request::{FileChange, FileUpdateType},
    packages::manifests::ManifestFile,
    result::Result,
    updater::traits::FileUpdater,
};

/// Handles package-lock.json file parsing and version updates for Node.js packages.
pub struct PackageLock {}

impl Default for PackageLock {
    fn default() -> Self {
        PackageLock::new()
    }
}

impl PackageLock {
    /// Create package-lock.json handler for version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update a single package-lock.json file
    fn update_lock_file(
        &self,
        manifest: &ManifestFile,
    ) -> Result<Option<FileChange>> {
        let mut lock_doc = self.load_doc(&manifest.content)?;

        // The lock's top-level `version` and its `packages[""]` entry both
        // describe the package the lock sits beside. In a workspace whose
        // root is not being released there is no such package, and writing
        // a member's version into them would misreport the root.
        if let Some(owner) = manifest.owner.as_ref() {
            lock_doc["version"] = json!(owner.tag.semver.to_string());
        }

        // Update packages section
        if let Some(packages) = lock_doc.get_mut("packages")
            && let Some(packages_obj) = packages.as_object_mut()
        {
            for (key, package_info) in packages_obj {
                if key.is_empty() {
                    if let Some(owner) = manifest.owner.as_ref() {
                        package_info["version"] =
                            json!(owner.tag.semver.to_string());
                    }

                    // Update dependencies within root package entry
                    if let Some(deps) = package_info.get_mut("dependencies")
                        && let Some(deps_obj) = deps.as_object_mut()
                    {
                        for ws_package in manifest.releasing.iter() {
                            if let Some((_, dep_info)) =
                                deps_obj.iter_mut().find(|(name, _)| {
                                    name.to_string() == ws_package.name
                                })
                            {
                                *dep_info = json!(format!(
                                    "{}",
                                    ws_package.tag.semver.to_string()
                                ));
                            }
                        }
                    }

                    // Update devDependencies within root package entry
                    if let Some(deps) = package_info.get_mut("devDependencies")
                        && let Some(deps_obj) = deps.as_object_mut()
                    {
                        for ws_package in manifest.releasing.iter() {
                            if let Some((_, dep_info)) =
                                deps_obj.iter_mut().find(|(name, _)| {
                                    name.to_string() == ws_package.name
                                })
                            {
                                *dep_info = json!(format!(
                                    "{}",
                                    ws_package.tag.semver.to_string()
                                ));
                            }
                        }
                    }

                    continue;
                }

                // Extract package name from node_modules/ key
                if let Some(package_name) = key.strip_prefix("node_modules/")
                    && let Some(ws_pkg) = manifest
                        .releasing
                        .iter()
                        .find(|p| p.name == package_name)
                {
                    package_info["version"] =
                        json!(ws_pkg.tag.semver.to_string());
                }
            }
        }

        let formatted_json = serde_json::to_string_pretty(&lock_doc)?;

        Ok(Some(FileChange {
            path: manifest.path.to_string_lossy().to_string(),
            content: formatted_json,
            update_type: FileUpdateType::Replace,
        }))
    }

    fn load_doc(&self, content: &str) -> Result<Value> {
        let doc = serde_json::from_str(content)?;
        Ok(doc)
    }
}

impl FileUpdater for PackageLock {
    /// Update version fields in package-lock.json files for all Node packages.
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename != "package-lock.json" {
            return Ok(None);
        }

        self.update_lock_file(manifest)
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
        let package_lock = PackageLock::new();
        let content =
            r#"{"name":"my-package","version":"1.0.0","packages":{}}"#;

        let manifest = ManifestFile {
            path: Path::new("package-lock.json").to_path_buf(),
            basename: "package-lock.json".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Node,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Node,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = package_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
    }

    #[test]
    fn updates_root_package_entry_version() {
        let package_lock = PackageLock::new();
        let content = r#"{
  "name": "my-package",
  "version": "1.0.0",
  "packages": {
    "": {
      "name": "my-package",
      "version": "1.0.0"
    }
  }
}"#;

        let manifest = ManifestFile {
            path: Path::new("package-lock.json").to_path_buf(),
            basename: "package-lock.json".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Node,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Node,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = package_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
        // Should appear twice: once at root, once in packages[""]
        assert_eq!(updated.matches("\"version\": \"2.0.0\"").count(), 2);
    }

    #[test]
    fn updates_workspace_dependencies_in_lock_file() {
        let package_lock = PackageLock::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "packages": {
    "": {
      "name": "package-a",
      "version": "1.0.0",
      "dependencies": {
        "package-b": "1.0.0"
      }
    }
  }
}"#;

        let package_b = ManifestPackage {
            name: "package-b".to_string(),
            release_type: ReleaseType::Node,
            tag: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::parse("3.0.0").unwrap(),
                sha: "def".into(),
                ..Tag::default()
            },
        };

        let manifest = ManifestFile {
            path: Path::new("package-lock.json").to_path_buf(),
            basename: "package-lock.json".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Node,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Node,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![package_b],
        };

        let result = package_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("\"package-b\": \"3.0.0\""));
    }

    #[test]
    fn updates_workspace_dev_dependencies_in_lock_file() {
        let package_lock = PackageLock::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "packages": {
    "": {
      "name": "package-a",
      "version": "1.0.0",
      "devDependencies": {
        "package-b": "1.0.0"
      }
    }
  }
}"#;

        let package_b = ManifestPackage {
            name: "package-b".to_string(),
            release_type: ReleaseType::Node,
            tag: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::parse("3.0.0").unwrap(),
                sha: "def".into(),
                ..Tag::default()
            },
        };

        let manifest = ManifestFile {
            path: Path::new("package-lock.json").to_path_buf(),
            basename: "package-lock.json".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Node,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Node,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![package_b],
        };

        let result = package_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("\"package-b\": \"3.0.0\""));
    }

    #[test]
    fn updates_node_modules_entries_for_workspace_packages() {
        let package_lock = PackageLock::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "packages": {
    "": {
      "name": "package-a",
      "version": "1.0.0"
    },
    "node_modules/package-b": {
      "version": "1.0.0"
    }
  }
}"#;

        let package_b = ManifestPackage {
            name: "package-b".to_string(),
            release_type: ReleaseType::Node,
            tag: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::parse("3.0.0").unwrap(),
                sha: "def".into(),
                ..Tag::default()
            },
        };

        let manifest = ManifestFile {
            path: Path::new("package-lock.json").to_path_buf(),
            basename: "package-lock.json".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Node,
            owner: Some(ManifestPackage {
                name: "package-a".to_string(),
                release_type: ReleaseType::Node,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![package_b],
        };

        let result = package_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        let parsed: Value = serde_json::from_str(&updated).unwrap();
        assert_eq!(
            parsed["packages"]["node_modules/package-b"]["version"],
            "3.0.0"
        );
    }

    #[test]
    fn handles_non_workspace_lock_files() {
        let package_lock = PackageLock::new();
        let content = r#"{
  "name": "my-package",
  "version": "1.0.0",
  "packages": {
    "": {
      "name": "my-package",
      "version": "1.0.0"
    }
  }
}"#;

        let manifest = ManifestFile {
            path: Path::new("package-lock.json").to_path_buf(),
            basename: "package-lock.json".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Node,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Node,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = package_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
    }

    #[test]
    fn process_package_returns_none_when_no_lock_files() {
        let package_lock = PackageLock::new();

        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".to_string(),
            content: r#"{"name":"my-package","version":"1.0.0"}"#.to_string(),
            release_type: ReleaseType::Node,
            owner: Some(ManifestPackage {
                name: "test".to_string(),
                release_type: ReleaseType::Node,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = package_lock.update(&manifest).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn preserves_other_fields_in_lock_file() {
        let package_lock = PackageLock::new();
        let content = r#"{
  "name": "my-package",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "requires": true,
  "packages": {
    "": {
      "name": "my-package",
      "version": "1.0.0"
    }
  }
}"#;

        let manifest = ManifestFile {
            path: Path::new("package-lock.json").to_path_buf(),
            basename: "package-lock.json".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Node,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Node,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = package_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
        assert!(updated.contains("\"lockfileVersion\": 2"));
        assert!(updated.contains("\"requires\": true"));
    }
}
