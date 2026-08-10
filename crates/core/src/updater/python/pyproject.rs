use toml_edit::{DocumentMut, value};

use crate::{
    forge::request::{FileChange, FileUpdateType},
    packages::manifests::ManifestFile,
    result::Result,
    updater::traits::FileUpdater,
};

pub struct PyProject {}

impl PyProject {
    pub fn new() -> Self {
        Self {}
    }

    fn load_doc(&self, content: &str) -> Result<DocumentMut> {
        let doc = content.parse::<DocumentMut>()?;
        Ok(doc)
    }
}

impl Default for PyProject {
    fn default() -> Self {
        PyProject::new()
    }
}

impl FileUpdater for PyProject {
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename != "pyproject.toml" {
            return Ok(None);
        }

        let Some(owner) = manifest.owner.as_ref() else {
            return Ok(None);
        };

        let mut doc = self.load_doc(&manifest.content)?;

        if let Some(project) = doc["project"].as_table_mut() {
            if project.get("dynamic").is_some() {
                log::info!(
                    "dynamic version found in pyproject.toml: skipping update"
                );
                return Ok(None);
            }

            log::info!(
                "updating {} project version to {}",
                manifest.path.to_string_lossy(),
                owner.tag.semver
            );

            project["version"] = value(owner.tag.semver.to_string());

            return Ok(Some(FileChange {
                path: manifest.path.to_string_lossy().to_string(),
                content: doc.to_string(),
                update_type: FileUpdateType::Replace,
            }));
        }

        if let Some(tool) = doc["tool"].as_table_mut()
            && let Some(project) = tool["poetry"].as_table_mut()
        {
            if project.get("dynamic").is_some() {
                log::info!(
                    "dynamic version found in pyproject.toml: skipping update"
                );
                return Ok(None);
            }

            log::info!(
                "updating {} tool.poetry version to {}",
                manifest.path.to_string_lossy(),
                owner.tag.semver
            );

            project["version"] = value(owner.tag.semver.to_string());

            return Ok(Some(FileChange {
                path: manifest.path.to_string_lossy().to_string(),
                content: doc.to_string(),
                update_type: FileUpdateType::Replace,
            }));
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
    fn updates_project_version() {
        let pyproject = PyProject::new();
        let content = r#"[project]
name = "my-package"
version = "1.0.0"
"#;

        let manifest = ManifestFile {
            path: Path::new("pyproject.toml").to_path_buf(),
            basename: "pyproject.toml".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Python,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Python,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = pyproject.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
    }

    #[test]
    fn updates_tool_poetry_version() {
        let pyproject = PyProject::new();
        let content = r#"[tool.poetry]
name = "my-package"
version = "1.0.0"
"#;

        let manifest = ManifestFile {
            path: Path::new("pyproject.toml").to_path_buf(),
            basename: "pyproject.toml".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Python,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Python,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = pyproject.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
    }

    #[test]
    fn skips_dynamic_version_in_project_section() {
        let pyproject = PyProject::new();
        let content = r#"[project]
name = "my-package"
version = "1.0.0"
dynamic = ["version"]
"#;

        let manifest = ManifestFile {
            path: Path::new("pyproject.toml").to_path_buf(),
            basename: "pyproject.toml".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Python,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Python,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = pyproject.update(&manifest).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn skips_dynamic_version_in_tool_poetry_section() {
        let pyproject = PyProject::new();
        let content = r#"[tool.poetry]
name = "my-package"
version = "1.0.0"
dynamic = ["version"]
"#;

        let manifest = ManifestFile {
            path: Path::new("pyproject.toml").to_path_buf(),
            basename: "pyproject.toml".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Python,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Python,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = pyproject.update(&manifest).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn preserves_other_fields() {
        let pyproject = PyProject::new();
        let content = r#"[project]
name = "my-package"
version = "1.0.0"
description = "A test package"
requires-python = ">=3.8"

[project.dependencies]
requests = "^2.28.0"
"#;

        let manifest = ManifestFile {
            path: Path::new("pyproject.toml").to_path_buf(),
            basename: "pyproject.toml".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Python,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Python,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = pyproject.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(updated.contains("description = \"A test package\""));
        assert!(updated.contains("requires-python = \">=3.8\""));
        assert!(updated.contains("requests = \"^2.28.0\""));
    }

    #[test]
    fn returns_none_when_no_project_or_poetry_sections() {
        let pyproject = PyProject::new();
        let content = r#"[build-system]
requires = ["setuptools", "wheel"]
"#;

        let manifest = ManifestFile {
            path: Path::new("pyproject.toml").to_path_buf(),
            basename: "pyproject.toml".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Python,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Python,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = pyproject.update(&manifest).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn process_package_returns_none_when_no_pyproject_files() {
        let pyproject = PyProject::new();

        let manifest = ManifestFile {
            path: Path::new("setup.py").to_path_buf(),
            basename: "setup.py".to_string(),
            content: "setup(name='my-package', version='1.0.0')".to_string(),
            release_type: ReleaseType::Python,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Python,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::new(2, 0, 0),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = pyproject.update(&manifest).unwrap();

        assert!(result.is_none());
    }
}
