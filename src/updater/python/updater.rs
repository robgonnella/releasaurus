//! Python updater for handling Python projects with various build systems and
//! package managers
use async_trait::async_trait;
use log::*;

use crate::{
    forge::{request::FileChange, traits::FileLoader},
    result::Result,
    updater::{
        framework::{Framework, Package},
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
        packages: Vec<Package>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let python_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Python))
            .collect::<Vec<Package>>();

        info!("Found {} python packages", python_packages.len());

        let mut file_changes: Vec<FileChange> = vec![];
        if let Some(changes) = self
            .pyproject
            .process_packages(&python_packages, loader)
            .await?
        {
            file_changes.extend(changes);
        }

        if let Some(changes) = self
            .setuppy
            .process_packages(&python_packages, loader)
            .await?
        {
            file_changes.extend(changes);
        }

        if let Some(changes) = self
            .setupcfg
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
    use semver::Version as SemVer;

    fn create_test_package(
        name: &str,
        path: &str,
        next_version: &str,
    ) -> Package {
        Package {
            name: name.to_string(),
            path: path.to_string(),
            framework: Framework::Python,
            next_version: Tag {
                sha: "test-sha".to_string(),
                name: format!("v{}", next_version),
                semver: SemVer::parse(next_version).unwrap(),
            },
        }
    }

    #[tokio::test]
    async fn test_update_pyproject_toml_project_section() {
        let updater = PythonUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let pyproject_toml = r#"[project]
name = "test-package"
version = "1.0.0"
description = "A test package"

[project.dependencies]
requests = "^2.28.0"
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pyproject.toml"))
            .times(1)
            .returning(move |_| Ok(Some(pyproject_toml.to_string())));

        // Mock for setup.py and setup.cfg (not found)
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.py"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/pyproject.toml");
        assert!(changes[0].content.contains("version = \"2.0.0\""));
        assert!(changes[0].content.contains("name = \"test-package\""));
    }

    #[tokio::test]
    async fn test_update_pyproject_toml_poetry_section() {
        let updater = PythonUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "3.0.0");

        let pyproject_toml = r#"[tool.poetry]
name = "test-package"
version = "1.0.0"
description = "A test package using Poetry"

[tool.poetry.dependencies]
python = "^3.8"
requests = "^2.28.0"
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pyproject.toml"))
            .times(1)
            .returning(move |_| Ok(Some(pyproject_toml.to_string())));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.py"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/pyproject.toml");
        assert!(changes[0].content.contains("version = \"3.0.0\""));
    }

    #[tokio::test]
    async fn test_update_pyproject_toml_skips_dynamic_version() {
        let updater = PythonUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let pyproject_toml = r#"[project]
name = "test-package"
dynamic = ["version"]
description = "A test package with dynamic version"
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pyproject.toml"))
            .times(1)
            .returning(move |_| Ok(Some(pyproject_toml.to_string())));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.py"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        // Should return None because dynamic version is skipped
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_setup_py() {
        let updater = PythonUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let setup_py = r#"from setuptools import setup, find_packages

setup(
    name="test-package",
    version = "1.0.0",
    description="A test package",
    packages=find_packages(),
)
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pyproject.toml"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.py"))
            .times(1)
            .returning(move |_| Ok(Some(setup_py.to_string())));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/setup.py");
        assert!(changes[0].content.contains("version = 2.0.0"));
        assert!(!changes[0].content.contains("version = \"1.0.0\""));
    }

    #[tokio::test]
    async fn test_update_setup_py_with_single_quotes() {
        let updater = PythonUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "3.0.0");

        let setup_py = r#"from setuptools import setup

setup(
    name='test-package',
    version = '1.0.0',
    description='A test package',
)
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pyproject.toml"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.py"))
            .times(1)
            .returning(move |_| Ok(Some(setup_py.to_string())));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].content.contains("version = 3.0.0"));
    }

    #[tokio::test]
    async fn test_update_setup_cfg() {
        let updater = PythonUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let setup_cfg = r#"[metadata]
name = test-package
version = 1.0.0
description = A test package

[options]
packages = find:
python_requires = >=3.8
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pyproject.toml"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.py"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(1)
            .returning(move |_| Ok(Some(setup_cfg.to_string())));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/setup.cfg");
        assert!(changes[0].content.contains("version = 2.0.0"));
        assert!(!changes[0].content.contains("version = 1.0.0"));
    }

    #[tokio::test]
    async fn test_update_multiple_files_in_one_package() {
        let updater = PythonUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let pyproject_toml = r#"[project]
name = "test-package"
version = "1.0.0"
"#;

        let setup_py = r#"setup(
    version = "1.0.0",
)
"#;

        let setup_cfg = r#"[metadata]
version = 1.0.0
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

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.py"))
            .times(1)
            .returning({
                let content = setup_py.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(1)
            .returning({
                let content = setup_cfg.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 3);

        // Verify all three files were updated
        assert!(
            changes
                .iter()
                .any(|c| c.path == "packages/test/pyproject.toml"
                    && c.content.contains("version = \"2.0.0\""))
        );
        assert!(changes.iter().any(|c| c.path == "packages/test/setup.py"
            && c.content.contains("version = 2.0.0")));
        assert!(changes.iter().any(|c| c.path == "packages/test/setup.cfg"
            && c.content.contains("version = 2.0.0")));
    }

    #[tokio::test]
    async fn test_update_multiple_packages() {
        let updater = PythonUpdater::new();
        let packages = vec![
            create_test_package("package-one", "packages/one", "2.0.0"),
            create_test_package("package-two", "packages/two", "3.0.0"),
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

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/one/setup.py"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/one/setup.cfg"))
            .times(1)
            .returning(|_| Ok(None));

        // Package two
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/two/pyproject.toml"))
            .times(1)
            .returning({
                let content = pyproject2.to_string();
                move |_| Ok(Some(content.clone()))
            });

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/two/setup.py"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/two/setup.cfg"))
            .times(1)
            .returning(|_| Ok(None));

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
            create_test_package("python-package", "packages/python", "2.0.0"),
            Package {
                name: "rust-package".to_string(),
                path: "packages/rust".to_string(),
                framework: Framework::Rust,
                next_version: Tag {
                    sha: "test-sha".to_string(),
                    name: "v1.0.0".to_string(),
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
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_setup_py_with_indentation() {
        let updater = PythonUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.5.0");

        let setup_py = r#"from setuptools import setup

setup(
    name="test-package",
    version = "1.0.0",
    description="Test",
    author="John Doe",
)
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pyproject.toml"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.py"))
            .times(1)
            .returning(move |_| Ok(Some(setup_py.to_string())));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        let content = &changes[0].content;
        assert!(content.contains("version = 2.5.0"));
        assert!(content.contains("author=\"John Doe\""));
    }

    #[tokio::test]
    async fn test_pyproject_toml_preserves_structure() {
        let updater = PythonUpdater::new();
        let package =
            create_test_package("test-package", "packages/test", "2.0.0");

        let pyproject_toml = r#"[build-system]
requires = ["setuptools>=42", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "test-package"
version = "1.0.0"
description = "A comprehensive test package"
readme = "README.md"
requires-python = ">=3.8"

[project.optional-dependencies]
dev = ["pytest>=7.0", "black>=22.0"]
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pyproject.toml"))
            .times(1)
            .returning(move |_| Ok(Some(pyproject_toml.to_string())));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.py"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        let content = &changes[0].content;

        // Version should be updated
        assert!(content.contains("version = \"2.0.0\""));

        // Structure should be preserved
        assert!(content.contains("[build-system]"));
        assert!(content.contains("[project]"));
        assert!(content.contains("readme = \"README.md\""));
        assert!(content.contains("[project.optional-dependencies]"));
        assert!(content.contains("pytest>=7.0"));
    }
}
