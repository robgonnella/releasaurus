//! Python updater for handling Python projects with various build systems and
//! package managers
use crate::{
    cli::Result,
    forge::request::FileChange,
    updater::{
        framework::UpdaterPackage,
        python::{pyproject::PyProject, setupcfg::SetupCfg, setuppy::SetupPy},
        traits::PackageUpdater,
    },
};

/// Updates Python package version files including pyproject.toml, setup.py,
/// and setup.cfg for various build systems.
pub struct PythonUpdater {
    pyproject: PyProject,
    setuppy: SetupPy,
    setupcfg: SetupCfg,
}

impl PythonUpdater {
    /// Create Python updater with handlers for multiple packaging formats.
    pub fn new() -> Self {
        Self {
            pyproject: PyProject::new(),
            setuppy: SetupPy::new(),
            setupcfg: SetupCfg::new(),
        }
    }
}

impl PackageUpdater for PythonUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        // workspaces not supported for python projects
        _workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        if let Some(changes) = self.pyproject.process_package(package)? {
            file_changes.extend(changes);
        } else if let Some(changes) = self.setupcfg.process_package(package)? {
            file_changes.extend(changes);
        } else if let Some(changes) = self.setuppy.process_package(package)? {
            file_changes.extend(changes);
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
        config::ManifestFile,
        test_helpers::create_test_tag,
        updater::framework::{Framework, UpdaterPackage},
    };

    #[tokio::test]
    async fn processes_python_project() {
        let updater = PythonUpdater::new();
        let content = r#"[project]
name = "my-package"
version = "1.0.0"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "pyproject.toml".to_string(),
            file_basename: "pyproject.toml".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_some());
        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[tokio::test]
    async fn returns_none_when_no_python_files() {
        let updater = PythonUpdater::new();
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "package.json".to_string(),
            file_basename: "package.json".to_string(),
            content: r#"{"version":"1.0.0"}"#.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_none());
    }
}
