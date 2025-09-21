use log::*;
use regex::Regex;
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::LazyLock,
};

use crate::{result::Result, updater::framework::Package};

static VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^(\s*version\s*=\s*[\"'][\w\.\-\+]+[\"'])"#).unwrap()
});

pub struct SetupPy {}

impl SetupPy {
    pub fn new() -> Self {
        Self {}
    }

    pub fn process_packages(&self, packages: &[Package]) -> Result<()> {
        for package in packages {
            let file_path = Path::new(&package.path).join("setup.py");

            if !file_path.exists() {
                info!(
                    "skipping: no setup.py detected for package: {}",
                    package.path
                );
                continue;
            }

            info!("found setup.py for package: {}", package.path);

            let updated_version =
                format!("version = {}", package.next_version.semver);

            let mut content = self.load_doc(&file_path)?;

            content =
                VERSION_REGEX.replace(&content, updated_version).to_string();

            self.write_doc(content, &file_path)?;
        }

        Ok(())
    }

    fn load_doc(&self, file_path: &PathBuf) -> Result<String> {
        let mut file = OpenOptions::new().read(true).open(file_path)?;
        let mut content = String::from("");
        file.read_to_string(&mut content)?;
        Ok(content)
    }

    fn write_doc(&self, content: String, file_path: &PathBuf) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(file_path)?;
        file.write_all(content.as_bytes())?;
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
    fn test_process_packages_updates_version() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("test-package");
        fs::create_dir_all(&package_path).unwrap();

        let setup_py_content = r#"from setuptools import setup, find_packages

setup(
    name="test-package",
    version="0.1.0",
    description="A test package",
    author="Test Author",
    author_email="test@example.com",
    packages=find_packages(),
    install_requires=[
        "requests>=2.25.0",
    ],
)
"#;

        fs::write(package_path.join("setup.py"), setup_py_content).unwrap();

        let packages = vec![create_test_package(
            "test-package",
            package_path.to_str().unwrap(),
            "1.2.3",
        )];

        let setup_py = SetupPy::new();
        let result = setup_py.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.py")).unwrap();
        assert!(updated_content.contains("version = 1.2.3"));
        assert!(!updated_content.contains("version=\"0.1.0\""));
        // Ensure other content is preserved
        assert!(updated_content.contains("from setuptools import setup"));
        assert!(updated_content.contains("name=\"test-package\""));
        assert!(updated_content.contains("author=\"Test Author\""));
    }

    #[test]
    fn test_process_packages_updates_quoted_version() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("quoted-package");
        fs::create_dir_all(&package_path).unwrap();

        let setup_py_content = r#"from setuptools import setup

setup(
    name="quoted-package",
    version="0.5.0",
    description="Package with quoted version",
)
"#;

        fs::write(package_path.join("setup.py"), setup_py_content).unwrap();

        let packages = vec![create_test_package(
            "quoted-package",
            package_path.to_str().unwrap(),
            "2.1.0",
        )];

        let setup_py = SetupPy::new();
        let result = setup_py.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.py")).unwrap();
        assert!(updated_content.contains("version = 2.1.0"));
        assert!(!updated_content.contains("version=\"0.5.0\""));
    }

    #[test]
    fn test_process_packages_updates_single_quoted_version() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("single-quoted-package");
        fs::create_dir_all(&package_path).unwrap();

        let setup_py_content = r#"from setuptools import setup

setup(
    name='single-quoted-package',
    version='1.0.0-beta.1',
    description='Package with single quoted version',
)
"#;

        fs::write(package_path.join("setup.py"), setup_py_content).unwrap();

        let packages = vec![create_test_package(
            "single-quoted-package",
            package_path.to_str().unwrap(),
            "1.0.0",
        )];

        let setup_py = SetupPy::new();
        let result = setup_py.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.py")).unwrap();
        assert!(updated_content.contains("version = 1.0.0"));
        assert!(!updated_content.contains("version='1.0.0-beta.1'"));
    }

    #[test]
    fn test_process_packages_updates_indented_version() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("indented-package");
        fs::create_dir_all(&package_path).unwrap();

        let setup_py_content = r#"from setuptools import setup

setup(
    name="indented-package",
        version="0.2.0",
    description="Package with indented version",
)
"#;

        fs::write(package_path.join("setup.py"), setup_py_content).unwrap();

        let packages = vec![create_test_package(
            "indented-package",
            package_path.to_str().unwrap(),
            "3.0.0",
        )];

        let setup_py = SetupPy::new();
        let result = setup_py.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.py")).unwrap();
        assert!(updated_content.contains("version = 3.0.0"));
        assert!(!updated_content.contains("version=\"0.2.0\""));
    }

    #[test]
    fn test_process_packages_skips_missing_setup_py() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("no-setup-py");
        fs::create_dir_all(&package_path).unwrap();

        // Create a pyproject.toml instead of setup.py
        let pyproject_content = r#"[project]
name = "no-setup-py"
version = "0.1.0"
"#;
        fs::write(package_path.join("pyproject.toml"), pyproject_content)
            .unwrap();

        let packages = vec![create_test_package(
            "no-setup-py",
            package_path.to_str().unwrap(),
            "1.0.0",
        )];

        let setup_py = SetupPy::new();
        let result = setup_py.process_packages(&packages);

        // Should succeed but skip the package
        assert!(result.is_ok());

        // setup.py should not be created
        assert!(!package_path.join("setup.py").exists());

        // pyproject.toml should remain unchanged
        let unchanged_content =
            fs::read_to_string(package_path.join("pyproject.toml")).unwrap();
        assert!(unchanged_content.contains("version = \"0.1.0\""));
    }

    #[test]
    fn test_process_packages_multiple_packages() {
        let temp_dir = TempDir::new().unwrap();

        let package1_path = temp_dir.path().join("package1");
        fs::create_dir_all(&package1_path).unwrap();
        let setup_py1_content = r#"setup(
    name="package1",
    version="0.1.0"
)"#;
        fs::write(package1_path.join("setup.py"), setup_py1_content).unwrap();

        let package2_path = temp_dir.path().join("package2");
        fs::create_dir_all(&package2_path).unwrap();
        let setup_py2_content = r#"setup(
    name="package2",
    version="0.2.0"
)"#;
        fs::write(package2_path.join("setup.py"), setup_py2_content).unwrap();

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

        let setup_py = SetupPy::new();
        let result = setup_py.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content1 =
            fs::read_to_string(package1_path.join("setup.py")).unwrap();
        assert!(updated_content1.contains("version = 1.0.0"));

        let updated_content2 =
            fs::read_to_string(package2_path.join("setup.py")).unwrap();
        assert!(updated_content2.contains("version = 2.0.0"));
    }

    #[test]
    fn test_version_regex_matches_various_formats() {
        let test_cases = vec![
            ("    version=\"1.0.0\"", true),
            ("version = '2.0.0'", true),
            ("  version = 3.0.0", false),
            ("    version=\"1.0.0-beta.1\"", true),
            ("version='2.0.0+build.1'", true),
            ("# version = \"commented\"", false),
            ("description = \"not version\"", false),
            ("some_version = \"1.0.0\"", false),
        ];

        for (input, should_match) in test_cases {
            let matches = VERSION_REGEX.is_match(input);
            assert_eq!(matches, should_match, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_process_packages_preserves_file_structure() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("structured-package");
        fs::create_dir_all(&package_path).unwrap();

        let setup_py_content = r#"#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
Setup script for structured-package.
"""

from setuptools import setup, find_packages

# Read README file
with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="structured-package",
    version="0.1.0",
    author="Test Author",
    author_email="test@example.com",
    description="A structured test package",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/example/structured-package",
    packages=find_packages(exclude=["tests"]),
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
    ],
    python_requires=">=3.8",
    install_requires=[
        "requests>=2.25.0",
        "click>=7.0",
    ],
    extras_require={
        "dev": ["pytest>=6.0", "black", "flake8"],
        "docs": ["sphinx", "sphinx-rtd-theme"],
    },
)
"#;

        fs::write(package_path.join("setup.py"), setup_py_content).unwrap();

        let packages = vec![create_test_package(
            "structured-package",
            package_path.to_str().unwrap(),
            "1.5.0",
        )];

        let setup_py = SetupPy::new();
        let result = setup_py.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.py")).unwrap();

        // Version should be updated
        assert!(updated_content.contains("version = 1.5.0"));
        assert!(!updated_content.contains("version=\"0.1.0\""));

        // All other content should be preserved
        assert!(updated_content.contains("#!/usr/bin/env python"));
        assert!(updated_content.contains("# -*- coding: utf-8 -*-"));
        assert!(
            updated_content.contains("Setup script for structured-package")
        );
        assert!(
            updated_content.contains(
                "with open(\"README.md\", \"r\", encoding=\"utf-8\")"
            )
        );
        assert!(updated_content.contains("name=\"structured-package\""));
        assert!(updated_content.contains("author=\"Test Author\""));
        assert!(updated_content.contains("install_requires=["));
        assert!(updated_content.contains("\"requests>=2.25.0\""));
        assert!(updated_content.contains("extras_require={"));
        assert!(updated_content.contains("python_requires=\">=3.8\""));
    }
}
