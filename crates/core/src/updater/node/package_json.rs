use serde_json::{Value, json};

use crate::{
    config::release_type::ReleaseType,
    forge::request::{FileChange, FileUpdateType},
    packages::manifests::{ManifestFile, ManifestPackage},
    result::Result,
    updater::traits::FileUpdater,
};

/// Handles package.json file parsing and version updates for Node.js packages.
pub struct PackageJson {}

impl Default for PackageJson {
    fn default() -> Self {
        PackageJson::new()
    }
}

impl PackageJson {
    /// Create package.json handler for version updates.
    pub fn new() -> Self {
        Self {}
    }

    fn update_deps(
        &self,
        doc: &mut Value,
        dep_type: &str,
        other_packages: &[ManifestPackage],
    ) -> Result<()> {
        if doc.get(dep_type).is_none() {
            return Ok(());
        }

        // Skip if this is a workspace package
        if let Some(workspaces) = doc.get("workspaces")
            && (workspaces.is_array() || workspaces.is_object())
        {
            log::debug!("skipping workspace package.json");
            return Ok(());
        }

        if let Some(deps) = doc[dep_type].as_object_mut() {
            for (dep_name, dep_value) in deps.clone() {
                // Skip workspace: and repo: protocol dependencies
                if let Some(version_str) = dep_value.as_str()
                    && (version_str.starts_with("workspace:")
                        || version_str.starts_with("repo:"))
                {
                    continue;
                }

                if let Some(package) =
                    other_packages.iter().find(|p| p.name == dep_name)
                {
                    deps[&dep_name] = json!(format!("^{}", package.tag.semver));
                }
            }
        }

        Ok(())
    }

    fn load_doc(&self, content: &str) -> Result<Value> {
        let doc = serde_json::from_str(content)?;
        Ok(doc)
    }
}

impl FileUpdater for PackageJson {
    /// Update version fields in package.json files for all Node packages.
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename != "package.json" {
            return Ok(None);
        }

        let Some(owner) = manifest.owner.as_ref() else {
            return Ok(None);
        };

        let mut doc = self.load_doc(&manifest.content)?;
        doc["version"] = json!(owner.tag.semver.to_string());

        let other_pkgs = manifest
            .releasing
            .iter()
            .filter(|p| {
                p.name != owner.name
                    && matches!(p.release_type, ReleaseType::Node)
            })
            .cloned()
            .collect::<Vec<ManifestPackage>>();

        self.update_deps(&mut doc, "dependencies", &other_pkgs)?;
        self.update_deps(&mut doc, "devDependencies", &other_pkgs)?;

        let formatted_json = serde_json::to_string_pretty(&doc)?;

        Ok(Some(FileChange {
            path: manifest.path.to_string_lossy().to_string(),
            content: formatted_json,
            update_type: FileUpdateType::Replace,
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        config::release_type::ReleaseType, forge::request::Tag,
        packages::manifests::ManifestFile,
    };

    use super::*;

    #[test]
    fn updates_version_field() {
        let package_json = PackageJson::new();
        let content = r#"{"name":"my-package","version":"1.0.0"}"#;

        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".to_string(),
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

        let result = package_json.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
    }

    #[test]
    fn updates_dependencies_to_workspace_packages() {
        let package_json = PackageJson::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "dependencies": {
    "package-b": "^1.0.0"
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
            path: Path::new("packages/a/package.json").to_path_buf(),
            basename: "package.json".to_string(),
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

        let result = package_json.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("\"package-b\": \"^3.0.0\""));
    }

    #[test]
    fn updates_dev_dependencies_to_workspace_packages() {
        let package_json = PackageJson::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "devDependencies": {
    "package-b": "^1.0.0"
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
            path: Path::new("packages/a/package.json").to_path_buf(),
            basename: "package.json".to_string(),
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

        let result = package_json.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("\"package-b\": \"^3.0.0\""));
    }

    #[test]
    fn skips_workspace_protocol_dependencies() {
        let package_json = PackageJson::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "dependencies": {
    "package-b": "workspace:^1.0.0"
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
            path: Path::new("packages/a/package.json").to_path_buf(),
            basename: "package.json".to_string(),
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

        let result = package_json.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("\"package-b\": \"workspace:^1.0.0\""));
    }

    #[test]
    fn skips_repo_protocol_dependencies() {
        let package_json = PackageJson::new();
        let content = r#"{
  "name": "package-a",
  "version": "1.0.0",
  "dependencies": {
    "package-b": "repo:^1.0.0"
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
            path: Path::new("packages/a/package.json").to_path_buf(),
            basename: "package.json".to_string(),
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

        let result = package_json.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("\"package-b\": \"repo:^1.0.0\""));
    }

    #[test]
    fn skips_workspace_root_package_json() {
        let package_json = PackageJson::new();
        let content = r#"{
  "name": "monorepo",
  "version": "1.0.0",
  "workspaces": ["packages/*"],
  "dependencies": {
    "package-a": "^1.0.0"
  }
}"#;

        let package_a = ManifestPackage {
            name: "package-a".to_string(),
            release_type: ReleaseType::Node,
            tag: Tag {
                name: "v3.0.0".into(),
                semver: semver::Version::parse("3.0.0").unwrap(),
                sha: "def".into(),
                ..Tag::default()
            },
        };

        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Node,
            owner: Some(ManifestPackage {
                name: "monorepo".to_string(),
                release_type: ReleaseType::Node,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![package_a],
        };

        let result = package_json.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("\"package-a\": \"^1.0.0\""));
    }

    #[test]
    fn process_package_returns_none_when_no_package_json_files() {
        let package_json = PackageJson::new();

        let manifest = ManifestFile {
            path: Path::new("Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".to_string(),
            content: "[package]\nversion = \"1.0.0\"".into(),
            release_type: ReleaseType::Node,
            owner: Some(ManifestPackage {
                name: "test".into(),
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

        let result = package_json.update(&manifest).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn preserves_other_fields_in_package_json() {
        let package_json = PackageJson::new();
        let content = r#"{
  "name": "my-package",
  "version": "1.0.0",
  "description": "A test package",
  "main": "index.js",
  "scripts": {
    "test": "jest"
  }
}"#;

        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".into(),
            content: content.into(),
            release_type: ReleaseType::Node,
            owner: Some(ManifestPackage {
                name: "my-package".into(),
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

        let result = package_json.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("\"version\": \"2.0.0\""));
        assert!(updated.contains("\"description\": \"A test package\""));
        assert!(updated.contains("\"main\": \"index.js\""));
        assert!(updated.contains("\"test\": \"jest\""));
    }
}
