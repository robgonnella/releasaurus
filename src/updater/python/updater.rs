//! Python updater for handling Python projects with various build systems and
//! package managers
use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        composite::CompositeUpdater,
        manager::UpdaterPackage,
        python::{pyproject::PyProject, setupcfg::SetupCfg, setuppy::SetupPy},
        traits::PackageUpdater,
    },
};

/// Updates Python package version files including pyproject.toml, setup.py,
/// and setup.cfg for various build systems.
pub struct PythonUpdater {
    composite: CompositeUpdater,
}

impl PythonUpdater {
    /// Create Python updater with handlers for multiple packaging formats.
    pub fn new() -> Self {
        Self {
            composite: CompositeUpdater::new(vec![
                Box::new(PyProject::new()),
                Box::new(SetupPy::new()),
                Box::new(SetupCfg::new()),
            ]),
        }
    }
}

impl PackageUpdater for PythonUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        self.composite.update(package, workspace_packages)
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
    fn processes_python_project() {
        let updater = PythonUpdater::new();
        let content = r#"[project]
name = "my-package"
version = "1.0.0"
"#;
        let manifest = ManifestFile {
            path: Path::new("pyproject.toml").to_path_buf(),
            basename: "pyproject.toml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_python_files() {
        let updater = PythonUpdater::new();
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
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }
}
