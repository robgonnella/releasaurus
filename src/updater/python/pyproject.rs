use log::*;
use toml_edit::{DocumentMut, value};

use crate::{
    Result,
    forge::request::{FileChange, FileUpdateType},
    updater::{manager::UpdaterPackage, traits::PackageUpdater},
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

impl PackageUpdater for PyProject {
    fn update(
        &self,
        package: &UpdaterPackage,
        _workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.basename != "pyproject.toml" {
                continue;
            }

            let mut doc = self.load_doc(&manifest.content)?;

            if let Some(project) = doc["project"].as_table_mut() {
                if project.get("dynamic").is_some() {
                    info!(
                        "dynamic version found in pyproject.toml: skipping update"
                    );
                    continue;
                }

                info!(
                    "updating {} project version to {}",
                    manifest.path, package.next_version.semver
                );

                project["version"] =
                    value(package.next_version.semver.to_string());

                file_changes.push(FileChange {
                    path: manifest.path.clone(),
                    content: doc.to_string(),
                    update_type: FileUpdateType::Replace,
                });

                continue;
            }

            if let Some(tool) = doc["tool"].as_table_mut()
                && let Some(project) = tool["poetry"].as_table_mut()
            {
                if project.get("dynamic").is_some() {
                    info!(
                        "dynamic version found in pyproject.toml: skipping update"
                    );
                    continue;
                }

                info!(
                    "updating {} tool.poetry version to {}",
                    manifest.path, package.next_version.semver
                );

                project["version"] =
                    value(package.next_version.semver.to_string());

                file_changes.push(FileChange {
                    path: manifest.path.clone(),
                    content: doc.to_string(),
                    update_type: FileUpdateType::Replace,
                });
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
    use std::rc::Rc;

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
    fn updates_project_version() {
        let pyproject = PyProject::new();
        let content = r#"[project]
name = "my-package"
version = "1.0.0"
"#;
        let manifest = ManifestFile {
            path: "pyproject.toml".to_string(),
            basename: "pyproject.toml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = pyproject.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
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
            path: "pyproject.toml".to_string(),
            basename: "pyproject.toml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = pyproject.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
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
            path: "pyproject.toml".to_string(),
            basename: "pyproject.toml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = pyproject.update(&package, &[]).unwrap();

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
            path: "pyproject.toml".to_string(),
            basename: "pyproject.toml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = pyproject.update(&package, &[]).unwrap();

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
            path: "pyproject.toml".to_string(),
            basename: "pyproject.toml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = pyproject.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
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
            path: "pyproject.toml".to_string(),
            basename: "pyproject.toml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = pyproject.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn process_package_handles_multiple_pyproject_files() {
        let pyproject = PyProject::new();
        let manifest1 = ManifestFile {
            path: "packages/a/pyproject.toml".to_string(),
            basename: "pyproject.toml".to_string(),
            content: "[project]\nname = \"package-a\"\nversion = \"1.0.0\"\n"
                .to_string(),
        };
        let manifest2 = ManifestFile {
            path: "packages/b/pyproject.toml".to_string(),
            basename: "pyproject.toml".to_string(),
            content: "[project]\nname = \"package-b\"\nversion = \"1.0.0\"\n"
                .to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = pyproject.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[test]
    fn process_package_returns_none_when_no_pyproject_files() {
        let pyproject = PyProject::new();
        let manifest = ManifestFile {
            path: "setup.py".to_string(),
            basename: "setup.py".to_string(),
            content: "setup(name='my-package', version='1.0.0')".to_string(),
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
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = pyproject.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }
}
