//! Python updater for handling Python projects with various build systems and
//! package managers
use color_eyre::eyre::Result;
use log::*;
use std::path::Path;

use crate::updater::framework::{Framework, Package};
use crate::updater::python::pyproject::PyProject;
use crate::updater::traits::PackageUpdater;

// For use later
// // Detect build system
// let build_system = if manifest_content.contains("[tool.poetry]")
// {
//     "poetry".to_string()
// } else if manifest_content.contains("[tool.setuptools]") {
//     "setuptools".to_string()
// } else if manifest_content.contains("[tool.flit]") {
//     "flit".to_string()
// } else {
//     "setuptools".to_string()
// };

// // Detect package manager
// let package_manager = if path.join("poetry.lock").exists() {
//     "poetry".to_string()
// } else if path.join("Pipfile").exists() {
//     "pipenv".to_string()
// } else {
//     "pip".to_string()
// };

// let metadata = PythonMetadata {
//     build_system,
//     package_manager,
//     uses_pyproject: true,
// };

// let metadata = PythonMetadata {
//     build_system: "setuptools".to_string(),
//     package_manager: "pip".to_string(),
//     uses_pyproject: false,
// };

/// Python updater - handles various Python packaging formats and build systems
pub struct PythonUpdater {
    pyproject: PyProject,
}

impl PythonUpdater {
    /// Create a new Python updater
    pub fn new() -> Self {
        Self {
            pyproject: PyProject::new(),
        }
    }
}

impl PackageUpdater for PythonUpdater {
    fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()> {
        let python_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Python))
            .collect::<Vec<Package>>();

        info!(
            "Found {} python packages in {}",
            python_packages.len(),
            root_path.display(),
        );

        self.pyproject.process_packages(&python_packages)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::types::Version as AnalyzerVersion;
    use semver::Version;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_package(
        name: &str,
        path: &str,
        version: &str,
        framework: Framework,
    ) -> Package {
        Package::new(
            name.to_string(),
            path.to_string(),
            AnalyzerVersion {
                tag: format!("v{}", version),
                semver: Version::parse(version).unwrap(),
            },
            framework,
        )
    }

    #[test]
    fn test_update_filters_python_packages_only() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create a Python package
        let python_package_path = root_path.join("python-pkg");
        fs::create_dir_all(&python_package_path).unwrap();
        fs::write(
            python_package_path.join("pyproject.toml"),
            r#"[project]
name = "python-pkg"
version = "0.1.0"
"#,
        )
        .unwrap();

        let packages = vec![
            create_test_package(
                "python-pkg",
                python_package_path.to_str().unwrap(),
                "1.0.0",
                Framework::Python,
            ),
            create_test_package(
                "rust-pkg",
                "rust-pkg",
                "2.0.0",
                Framework::Rust,
            ),
            create_test_package(
                "node-pkg",
                "node-pkg",
                "3.0.0",
                Framework::Node,
            ),
            create_test_package(
                "generic-pkg",
                "generic-pkg",
                "4.0.0",
                Framework::Generic,
            ),
        ];

        let updater = PythonUpdater::new();
        let result = updater.update(root_path, packages);

        assert!(result.is_ok());

        // Verify that only the Python package was updated
        let updated_content =
            fs::read_to_string(python_package_path.join("pyproject.toml"))
                .unwrap();
        assert!(updated_content.contains("version = \"1.0.0\""));
    }

    #[test]
    fn test_update_with_no_python_packages() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        let packages = vec![
            create_test_package(
                "rust-pkg",
                "rust-pkg",
                "1.0.0",
                Framework::Rust,
            ),
            create_test_package(
                "node-pkg",
                "node-pkg",
                "2.0.0",
                Framework::Node,
            ),
        ];

        let updater = PythonUpdater::new();
        let result = updater.update(root_path, packages);

        // Should succeed even with no Python packages
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_with_empty_packages() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        let packages = vec![];

        let updater = PythonUpdater::new();
        let result = updater.update(root_path, packages);

        // Should succeed with empty package list
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_multiple_python_packages() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create first Python package
        let package1_path = root_path.join("package1");
        fs::create_dir_all(&package1_path).unwrap();
        fs::write(
            package1_path.join("pyproject.toml"),
            r#"[project]
name = "package1"
version = "0.1.0"
"#,
        )
        .unwrap();

        // Create second Python package
        let package2_path = root_path.join("package2");
        fs::create_dir_all(&package2_path).unwrap();
        fs::write(
            package2_path.join("pyproject.toml"),
            r#"[tool.poetry]
name = "package2"
version = "0.2.0"
"#,
        )
        .unwrap();

        let packages = vec![
            create_test_package(
                "package1",
                package1_path.to_str().unwrap(),
                "1.0.0",
                Framework::Python,
            ),
            create_test_package(
                "package2",
                package2_path.to_str().unwrap(),
                "2.0.0",
                Framework::Python,
            ),
            create_test_package(
                "rust-pkg",
                "rust-pkg",
                "3.0.0",
                Framework::Rust,
            ),
        ];

        let updater = PythonUpdater::new();
        let result = updater.update(root_path, packages);

        assert!(result.is_ok());

        // Verify both Python packages were updated
        let updated_content1 =
            fs::read_to_string(package1_path.join("pyproject.toml")).unwrap();
        assert!(updated_content1.contains("version = \"1.0.0\""));

        let updated_content2 =
            fs::read_to_string(package2_path.join("pyproject.toml")).unwrap();
        assert!(updated_content2.contains("version = \"2.0.0\""));
    }

    #[test]
    fn test_update_propagates_pyproject_errors() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create a package with malformed pyproject.toml
        let package_path = root_path.join("malformed-pkg");
        fs::create_dir_all(&package_path).unwrap();
        fs::write(
            package_path.join("pyproject.toml"),
            r#"[project
name = "malformed-pkg"
version = "0.1.0"
"#, // Missing closing bracket
        )
        .unwrap();

        let packages = vec![create_test_package(
            "malformed-pkg",
            package_path.to_str().unwrap(),
            "1.0.0",
            Framework::Python,
        )];

        let updater = PythonUpdater::new();
        let result = updater.update(root_path, packages);

        // Should propagate the error from PyProject
        assert!(result.is_err());
    }

    #[test]
    fn test_updater_new() {
        let updater = PythonUpdater::new();

        // Just verify we can create a new updater without panicking
        // The internal PyProject should be initialized
        assert!(!std::ptr::eq(
            &updater.pyproject as *const _,
            std::ptr::null()
        ));
    }

    #[test]
    fn test_update_with_missing_pyproject_files() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create package directory but no pyproject.toml
        let package_path = root_path.join("missing-pyproject");
        fs::create_dir_all(&package_path).unwrap();

        let packages = vec![create_test_package(
            "missing-pyproject",
            package_path.to_str().unwrap(),
            "1.0.0",
            Framework::Python,
        )];

        let updater = PythonUpdater::new();
        let result = updater.update(root_path, packages);

        // Should succeed (PyProject handles missing files gracefully)
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_with_complex_version_numbers() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        let package_path = root_path.join("complex-version");
        fs::create_dir_all(&package_path).unwrap();
        fs::write(
            package_path.join("pyproject.toml"),
            r#"[project]
name = "complex-version"
version = "0.1.0"
"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "complex-version",
            package_path.to_str().unwrap(),
            "1.0.0-alpha.1",
            Framework::Python,
        )];

        let updater = PythonUpdater::new();
        let result = updater.update(root_path, packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("pyproject.toml")).unwrap();
        assert!(updated_content.contains("version = \"1.0.0-alpha.1\""));
    }
}
