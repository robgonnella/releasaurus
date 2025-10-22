//! Python updater for handling Python projects with various build systems and
//! package managers
use async_trait::async_trait;
use log::*;

use crate::{
    forge::{request::FileChange, traits::FileLoader},
    result::Result,
    updater::{
        framework::{Framework, UpdaterPackage},
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

#[async_trait]
impl PackageUpdater for PythonUpdater {
    async fn update(
        &self,
        packages: Vec<UpdaterPackage>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let python_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Python))
            .collect::<Vec<UpdaterPackage>>();

        info!("Found {} python packages", python_packages.len());

        if python_packages.is_empty() {
            return Ok(None);
        }

        let mut file_changes: Vec<FileChange> = vec![];

        if let Some(changes) = self
            .pyproject
            .process_packages(&python_packages, loader)
            .await?
        {
            file_changes.extend(changes);
        } else if let Some(changes) = self
            .setupcfg
            .process_packages(&python_packages, loader)
            .await?
        {
            file_changes.extend(changes);
        } else if let Some(changes) = self
            .setuppy
            .process_packages(&python_packages, loader)
            .await?
        {
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
    use crate::analyzer::release::Tag;
    use crate::forge::traits::MockFileLoader;
    use crate::test_helpers::create_test_updater_package;
    use semver::Version as SemVer;

    #[tokio::test]
    async fn test_update_prioritizes_pyproject_toml() {
        let updater = PythonUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
            Framework::Python,
        );

        let pyproject_toml = r#"[project]
name = "test-package"
version = "1.0.0"
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pyproject.toml"))
            .times(1)
            .returning({
                let content = pyproject_toml.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // setup.py and setup.cfg won't be checked since pyproject.toml is found first
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.py"))
            .times(0);

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(0);

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        // Only pyproject.toml should be updated (takes priority)
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/pyproject.toml");
        assert!(changes[0].content.contains("version = \"2.0.0\""));
    }

    #[tokio::test]
    async fn test_update_multiple_packages() {
        let updater = PythonUpdater::new();
        let packages = vec![
            create_test_updater_package(
                "package-one",
                "packages/one",
                "2.0.0",
                Framework::Python,
            ),
            create_test_updater_package(
                "package-two",
                "packages/two",
                "3.0.0",
                Framework::Python,
            ),
        ];

        let pyproject1 = r#"[project]
name = "package-one"
version = "1.0.0"
"#;

        let pyproject2 = r#"[tool.poetry]
name = "package-two"
version = "1.0.0"
"#;

        let mut mock_loader = MockFileLoader::new();

        // Package one
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/one/pyproject.toml"))
            .times(1)
            .returning({
                let content = pyproject1.to_string();
                move |_| Ok(Some(content.clone()))
            });

        // Package two
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/two/pyproject.toml"))
            .times(1)
            .returning({
                let content = pyproject2.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);

        // Check first package
        let change1 = changes
            .iter()
            .find(|c| c.path == "packages/one/pyproject.toml")
            .unwrap();
        assert!(change1.content.contains("version = \"2.0.0\""));

        // Check second package
        let change2 = changes
            .iter()
            .find(|c| c.path == "packages/two/pyproject.toml")
            .unwrap();
        assert!(change2.content.contains("version = \"3.0.0\""));
    }

    #[tokio::test]
    async fn test_update_filters_python_packages() {
        let updater = PythonUpdater::new();

        let packages = vec![
            create_test_updater_package(
                "python-package",
                "packages/python",
                "2.0.0",
                Framework::Python,
            ),
            UpdaterPackage {
                name: "rust-package".into(),
                path: "packages/rust".into(),
                workspace_root: ".".into(),
                framework: Framework::Rust,
                next_version: Tag {
                    sha: "test-sha".into(),
                    name: "v1.0.0".into(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                },
            },
        ];

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();

        // Should return None when no Python files are found
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_no_files_found() {
        let updater = PythonUpdater::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
            Framework::Python,
        );

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_none());
    }
}
