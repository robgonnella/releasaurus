use log::*;
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use toml_edit::{DocumentMut, value};

use crate::{result::Result, updater::framework::Package};

pub struct PyProject {}

impl PyProject {
    pub fn new() -> Self {
        Self {}
    }

    pub fn process_packages(&self, packages: &[Package]) -> Result<()> {
        for package in packages {
            let file_path = Path::new(&package.path).join("pyproject.toml");

            if !file_path.exists() {
                info!(
                    "skipping: no pyproject.toml detected for package: {}",
                    package.path
                );
                continue;
            }

            info!("found pyproject.toml for package: {}", package.path);

            let mut doc = self.load_doc(&file_path)?;

            if let Some(project) = doc["project"].as_table_mut() {
                if project.get("dynamic").is_some() {
                    info!(
                        "dynamic version found in pyproject.toml: skipping update"
                    );
                    continue;
                }

                info!(
                    "updating {} project version to {}",
                    file_path.display(),
                    package.next_version.semver
                );

                project["version"] =
                    value(package.next_version.semver.to_string());

                self.write_doc(&mut doc, &file_path)?;

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
                    file_path.display(),
                    package.next_version.semver
                );

                project["version"] =
                    value(package.next_version.semver.to_string());

                self.write_doc(&mut doc, &file_path)?;
            }
        }

        Ok(())
    }

    fn load_doc(&self, file_path: &PathBuf) -> Result<DocumentMut> {
        let mut file = OpenOptions::new().read(true).open(file_path)?;
        let mut content = String::from("");
        file.read_to_string(&mut content)?;
        let doc = content.parse::<DocumentMut>()?;
        Ok(doc)
    }

    fn write_doc(
        &self,
        doc: &mut DocumentMut,
        file_path: &PathBuf,
    ) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(file_path)?;
        file.write_all(doc.to_string().as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::release::Tag;
    use crate::updater::framework::Framework;
    use semver::Version;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_package(name: &str, path: &str, version: &str) -> Package {
        Package::new(
            name.to_string(),
            path.to_string(),
            Tag {
                sha: "abc123".into(),
                name: format!("v{}", version),
                semver: Version::parse(version).unwrap(),
            },
            Framework::Python,
        )
    }

    #[test]
    fn test_process_packages_with_project_section() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("test-package");
        fs::create_dir_all(&package_path).unwrap();

        let pyproject_content = r#"[build-system]
requires = ["setuptools", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "test-package"
version = "0.1.0"
description = "A test package"
authors = [
    {name = "Test Author", email = "test@example.com"},
]
dependencies = [
    "requests>=2.25.0",
]

[project.urls]
Homepage = "https://example.com"
"#;

        fs::write(package_path.join("pyproject.toml"), pyproject_content)
            .unwrap();

        let packages = vec![create_test_package(
            "test-package",
            package_path.to_str().unwrap(),
            "1.2.3",
        )];

        let pyproject = PyProject::new();
        let result = pyproject.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("pyproject.toml")).unwrap();
        assert!(updated_content.contains("version = \"1.2.3\""));
        assert!(!updated_content.contains("version = \"0.1.0\""));
    }

    #[test]
    fn test_process_packages_with_poetry_section() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("poetry-package");
        fs::create_dir_all(&package_path).unwrap();

        let pyproject_content = r#"[build-system]
requires = ["poetry-core>=1.0.0"]
build-backend = "poetry.core.masonry.api"

[tool.poetry]
name = "poetry-package"
version = "0.5.0"
description = "A poetry package"
authors = ["Test Author <test@example.com>"]

[tool.poetry.dependencies]
python = "^3.8"
requests = "^2.25.0"
"#;

        fs::write(package_path.join("pyproject.toml"), pyproject_content)
            .unwrap();

        let packages = vec![create_test_package(
            "poetry-package",
            package_path.to_str().unwrap(),
            "2.0.0",
        )];

        let pyproject = PyProject::new();
        let result = pyproject.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("pyproject.toml")).unwrap();

        assert!(updated_content.contains("version = \"2.0.0\""));
        assert!(!updated_content.contains("version = \"0.5.0\""));
    }

    #[test]
    fn test_skip_dynamic_version_in_project_section() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("dynamic-package");
        fs::create_dir_all(&package_path).unwrap();

        let pyproject_content = r#"[build-system]
requires = ["setuptools", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "dynamic-package"
dynamic = ["version"]
description = "A package with dynamic version"
authors = [
    {name = "Test Author", email = "test@example.com"},
]
"#;

        fs::write(package_path.join("pyproject.toml"), pyproject_content)
            .unwrap();

        let packages = vec![create_test_package(
            "dynamic-package",
            package_path.to_str().unwrap(),
            "1.0.0",
        )];

        let pyproject = PyProject::new();
        let result = pyproject.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("pyproject.toml")).unwrap();
        // Should remain unchanged
        assert!(updated_content.contains(r#"dynamic = ["version"]"#));
        assert!(!updated_content.contains("version = \"1.0.0\""));
    }

    #[test]
    fn test_skip_dynamic_version_in_poetry_section() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("dynamic-poetry");
        fs::create_dir_all(&package_path).unwrap();

        let pyproject_content = r#"[build-system]
requires = ["poetry-core>=1.0.0"]
build-backend = "poetry.core.masonry.api"

[tool.poetry]
name = "dynamic-poetry"
dynamic = ["version"]
description = "A poetry package with dynamic version"
authors = ["Test Author <test@example.com>"]
"#;

        fs::write(package_path.join("pyproject.toml"), pyproject_content)
            .unwrap();

        let packages = vec![create_test_package(
            "dynamic-poetry",
            package_path.to_str().unwrap(),
            "1.0.0",
        )];

        let pyproject = PyProject::new();
        let result = pyproject.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("pyproject.toml")).unwrap();
        // Should remain unchanged
        assert!(updated_content.contains(r#"dynamic = ["version"]"#));
        assert!(!updated_content.contains("version = \"1.0.0\""));
    }

    #[test]
    fn test_missing_pyproject_toml() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("no-pyproject");
        fs::create_dir_all(&package_path).unwrap();
        // Intentionally not creating pyproject.toml

        let packages = vec![create_test_package(
            "no-pyproject",
            package_path.to_str().unwrap(),
            "1.0.0",
        )];

        let pyproject = PyProject::new();
        let result = pyproject.process_packages(&packages);

        // Should succeed but skip the package
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_multiple_packages() {
        let temp_dir = TempDir::new().unwrap();

        // Create first package with project section
        let package1_path = temp_dir.path().join("package1");
        fs::create_dir_all(&package1_path).unwrap();
        let pyproject1_content = r#"[project]
name = "package1"
version = "0.1.0"
"#;
        fs::write(package1_path.join("pyproject.toml"), pyproject1_content)
            .unwrap();

        // Create second package with poetry section
        let package2_path = temp_dir.path().join("package2");
        fs::create_dir_all(&package2_path).unwrap();
        let pyproject2_content = r#"[tool.poetry]
name = "package2"
version = "0.2.0"
"#;
        fs::write(package2_path.join("pyproject.toml"), pyproject2_content)
            .unwrap();

        let packages = vec![
            create_test_package(
                "package1",
                package1_path.to_str().unwrap(),
                "1.0.0",
            ),
            create_test_package(
                "package2",
                package2_path.to_str().unwrap(),
                "2.0.0",
            ),
        ];

        let pyproject = PyProject::new();
        let result = pyproject.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content1 =
            fs::read_to_string(package1_path.join("pyproject.toml")).unwrap();
        assert!(updated_content1.contains("version = \"1.0.0\""));

        let updated_content2 =
            fs::read_to_string(package2_path.join("pyproject.toml")).unwrap();
        assert!(updated_content2.contains("version = \"2.0.0\""));
    }

    #[test]
    fn test_malformed_toml_error() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("malformed-package");
        fs::create_dir_all(&package_path).unwrap();

        let malformed_content = r#"[project
name = "malformed-package"
version = "0.1.0"
"#; // Missing closing bracket

        fs::write(package_path.join("pyproject.toml"), malformed_content)
            .unwrap();

        let packages = vec![create_test_package(
            "malformed-package",
            package_path.to_str().unwrap(),
            "1.0.0",
        )];

        let pyproject = PyProject::new();
        let result = pyproject.process_packages(&packages);

        // Should return an error for malformed TOML
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_pyproject_toml() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("empty-package");
        fs::create_dir_all(&package_path).unwrap();

        fs::write(package_path.join("pyproject.toml"), "").unwrap();

        let packages = vec![create_test_package(
            "empty-package",
            package_path.to_str().unwrap(),
            "1.0.0",
        )];

        let pyproject = PyProject::new();
        let result = pyproject.process_packages(&packages);

        // Should succeed but not modify anything
        assert!(result.is_ok());

        let content =
            fs::read_to_string(package_path.join("pyproject.toml")).unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn test_preserve_toml_formatting() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("format-package");
        fs::create_dir_all(&package_path).unwrap();

        let pyproject_content = r#"# This is a comment
[build-system]
requires = ["setuptools"]

# Another comment
[project]
name = "format-package"
version = "0.1.0"  # inline comment
description = "Test formatting preservation"

# Final comment
"#;

        fs::write(package_path.join("pyproject.toml"), pyproject_content)
            .unwrap();

        let packages = vec![create_test_package(
            "format-package",
            package_path.to_str().unwrap(),
            "1.5.0",
        )];

        let pyproject = PyProject::new();
        let result = pyproject.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("pyproject.toml")).unwrap();

        // Version should be updated
        assert!(updated_content.contains("version = \"1.5.0\""));
        assert!(!updated_content.contains("version = \"0.1.0\""));

        // Comments should be preserved
        assert!(updated_content.contains("# This is a comment"));
        assert!(updated_content.contains("# Another comment"));
        assert!(updated_content.contains("# Final comment"));

        // Structure should be preserved
        assert!(updated_content.contains("[build-system]"));
        assert!(updated_content.contains("requires = [\"setuptools\"]"));
    }
}
