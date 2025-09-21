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
    Regex::new(r#"(?m)^(\s*version\s*=\s*[\"']?[\w\.\-\+]+[\"']?)"#).unwrap()
});

pub struct SetupCfg {}

impl SetupCfg {
    pub fn new() -> Self {
        Self {}
    }

    pub fn process_packages(&self, packages: &[Package]) -> Result<()> {
        for package in packages {
            let file_path = Path::new(&package.path).join("setup.cfg");

            if !file_path.exists() {
                info!(
                    "skipping: no setup.cfg detected for package: {}",
                    package.path
                );
                continue;
            }

            info!("found setup.cfg for package: {}", package.path);

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
    fn test_process_packages_with_quoted_version() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("quoted-package");
        fs::create_dir_all(&package_path).unwrap();

        let setup_cfg_content = r#"[metadata]
name = quoted-package
version = "0.1.0"
author = Test Author
author_email = test@example.com
description = A test package with quoted version

[options]
packages = find:
python_requires = >=3.8
install_requires =
    requests>=2.25.0
"#;

        fs::write(package_path.join("setup.cfg"), setup_cfg_content).unwrap();

        let packages = vec![create_test_package(
            "quoted-package",
            package_path.to_str().unwrap(),
            "1.2.3",
        )];

        let setupcfg = SetupCfg::new();
        let result = setupcfg.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.cfg")).unwrap();
        assert!(updated_content.contains("version = 1.2.3"));
        assert!(!updated_content.contains(r#"version = "0.1.0""#));
        // Should preserve other content
        assert!(updated_content.contains("name = quoted-package"));
        assert!(updated_content.contains("python_requires = >=3.8"));
    }

    #[test]
    fn test_process_packages_with_unquoted_version() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("unquoted-package");
        fs::create_dir_all(&package_path).unwrap();

        let setup_cfg_content = r#"[metadata]
name = unquoted-package
version = 0.5.0
description = A test package with unquoted version

[options]
packages = find:
"#;

        fs::write(package_path.join("setup.cfg"), setup_cfg_content).unwrap();

        let packages = vec![create_test_package(
            "unquoted-package",
            package_path.to_str().unwrap(),
            "2.0.0",
        )];

        let setupcfg = SetupCfg::new();
        let result = setupcfg.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.cfg")).unwrap();
        assert!(updated_content.contains("version = 2.0.0"));
        assert!(!updated_content.contains("version = 0.5.0"));
    }

    #[test]
    fn test_process_packages_with_single_quotes() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("single-quote-package");
        fs::create_dir_all(&package_path).unwrap();

        let setup_cfg_content = r#"[metadata]
name = single-quote-package
version = '0.3.0'
author = Test Author
"#;

        fs::write(package_path.join("setup.cfg"), setup_cfg_content).unwrap();

        let packages = vec![create_test_package(
            "single-quote-package",
            package_path.to_str().unwrap(),
            "1.5.0",
        )];

        let setupcfg = SetupCfg::new();
        let result = setupcfg.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.cfg")).unwrap();
        assert!(updated_content.contains("version = 1.5.0"));
        assert!(!updated_content.contains("version = '0.3.0'"));
    }

    #[test]
    fn test_process_packages_with_spaces_around_equals() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("spaced-package");
        fs::create_dir_all(&package_path).unwrap();

        let setup_cfg_content = r#"[metadata]
name = spaced-package
version   =   0.1.0
description = Package with spaces around equals
"#;

        fs::write(package_path.join("setup.cfg"), setup_cfg_content).unwrap();

        let packages = vec![create_test_package(
            "spaced-package",
            package_path.to_str().unwrap(),
            "1.0.0",
        )];

        let setupcfg = SetupCfg::new();
        let result = setupcfg.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.cfg")).unwrap();
        assert!(updated_content.contains("version = 1.0.0"));
        assert!(!updated_content.contains("version   =   0.1.0"));
    }

    #[test]
    fn test_process_packages_missing_setup_cfg() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("no-setup-cfg");
        fs::create_dir_all(&package_path).unwrap();
        // Intentionally not creating setup.cfg

        let packages = vec![create_test_package(
            "no-setup-cfg",
            package_path.to_str().unwrap(),
            "1.0.0",
        )];

        let setupcfg = SetupCfg::new();
        let result = setupcfg.process_packages(&packages);

        // Should succeed but skip the package
        assert!(result.is_ok());
        assert!(!package_path.join("setup.cfg").exists());
    }

    #[test]
    fn test_process_multiple_packages() {
        let temp_dir = TempDir::new().unwrap();

        // Create first package
        let package1_path = temp_dir.path().join("package1");
        fs::create_dir_all(&package1_path).unwrap();
        let setup_cfg1_content = r#"[metadata]
name = package1
version = 0.1.0
"#;
        fs::write(package1_path.join("setup.cfg"), setup_cfg1_content).unwrap();

        // Create second package
        let package2_path = temp_dir.path().join("package2");
        fs::create_dir_all(&package2_path).unwrap();
        let setup_cfg2_content = r#"[metadata]
name = package2
version = "0.2.0"
"#;
        fs::write(package2_path.join("setup.cfg"), setup_cfg2_content).unwrap();

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

        let setupcfg = SetupCfg::new();
        let result = setupcfg.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content1 =
            fs::read_to_string(package1_path.join("setup.cfg")).unwrap();
        assert!(updated_content1.contains("version = 1.0.0"));

        let updated_content2 =
            fs::read_to_string(package2_path.join("setup.cfg")).unwrap();
        assert!(updated_content2.contains("version = 2.0.0"));
    }

    #[test]
    fn test_process_packages_with_complex_version() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("complex-version");
        fs::create_dir_all(&package_path).unwrap();

        let setup_cfg_content = r#"[metadata]
name = complex-version
version = 0.1.0
"#;

        fs::write(package_path.join("setup.cfg"), setup_cfg_content).unwrap();

        let packages = vec![create_test_package(
            "complex-version",
            package_path.to_str().unwrap(),
            "1.0.0-alpha.1",
        )];

        let setupcfg = SetupCfg::new();
        let result = setupcfg.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.cfg")).unwrap();
        assert!(updated_content.contains("version = 1.0.0-alpha.1"));
    }

    #[test]
    fn test_process_packages_preserves_formatting() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("format-package");
        fs::create_dir_all(&package_path).unwrap();

        let setup_cfg_content = r#"# This is a comment
[metadata]
name = format-package
version = 0.1.0
description = Test formatting preservation
author = Test Author

# Another section
[options]
packages = find:
python_requires = >=3.8

# Final comment
"#;

        fs::write(package_path.join("setup.cfg"), setup_cfg_content).unwrap();

        let packages = vec![create_test_package(
            "format-package",
            package_path.to_str().unwrap(),
            "1.5.0",
        )];

        let setupcfg = SetupCfg::new();
        let result = setupcfg.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.cfg")).unwrap();

        // Version should be updated
        assert!(updated_content.contains("version = 1.5.0"));
        assert!(!updated_content.contains("version = 0.1.0"));

        // Comments and formatting should be preserved
        assert!(updated_content.contains("# This is a comment"));
        assert!(updated_content.contains("# Another section"));
        assert!(updated_content.contains("# Final comment"));
        assert!(updated_content.contains("[options]"));
        assert!(updated_content.contains("python_requires = >=3.8"));
    }

    #[test]
    fn test_process_packages_no_version_found() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("no-version");
        fs::create_dir_all(&package_path).unwrap();

        let setup_cfg_content = r#"[metadata]
name = no-version
description = Package without version

[options]
packages = find:
"#;

        fs::write(package_path.join("setup.cfg"), setup_cfg_content).unwrap();

        let packages = vec![create_test_package(
            "no-version",
            package_path.to_str().unwrap(),
            "1.0.0",
        )];

        let setupcfg = SetupCfg::new();
        let result = setupcfg.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.cfg")).unwrap();
        // Should remain unchanged since no version was found to replace
        assert_eq!(updated_content, setup_cfg_content);
    }

    #[test]
    fn test_process_packages_with_empty_setup_cfg() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("empty-setup");
        fs::create_dir_all(&package_path).unwrap();

        fs::write(package_path.join("setup.cfg"), "").unwrap();

        let packages = vec![create_test_package(
            "empty-setup",
            package_path.to_str().unwrap(),
            "1.0.0",
        )];

        let setupcfg = SetupCfg::new();
        let result = setupcfg.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.cfg")).unwrap();
        assert!(updated_content.is_empty());
    }

    #[test]
    fn test_setupcfg_new() {
        let setupcfg = SetupCfg::new();
        // Just verify we can create a new SetupCfg without panicking
        assert!(!std::ptr::eq(&setupcfg as *const _, std::ptr::null()));
    }

    #[test]
    fn test_version_regex_patterns() {
        // Test the regex directly with various version formats
        let test_cases = vec![
            ("version = 1.0.0", "version = 2.0.0", true),
            ("version = \"1.0.0\"", "version = 2.0.0", true),
            ("version = '1.0.0'", "version = 2.0.0", true),
            ("version   =   1.0.0", "version = 2.0.0", true),
            ("version=1.0.0", "version = 2.0.0", true),
            ("version = 1.0.0-alpha.1", "version = 2.0.0", true),
            ("  version = 1.0.0", "version = 2.0.0", true), // indented version
            ("name = test-package", "name = test-package", false), // Should not match
            ("# version = 1.0.0", "# version = 1.0.0", false), // Should not match comments
        ];

        for (input, expected_after_replacement, should_match) in test_cases {
            let result =
                VERSION_REGEX.replace(input, "version = 2.0.0").to_string();
            if should_match {
                assert_eq!(
                    result, expected_after_replacement,
                    "Failed for input: {}",
                    input
                );
            } else {
                assert_eq!(result, input, "Should not have changed: {}", input);
            }
        }
    }

    #[test]
    fn test_process_packages_with_version_in_comments() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("comment-version");
        fs::create_dir_all(&package_path).unwrap();

        let setup_cfg_content = r#"[metadata]
name = comment-version
# version = 0.0.1 (this should not be changed)
version = 0.1.0
description = Package with version in comments
"#;

        fs::write(package_path.join("setup.cfg"), setup_cfg_content).unwrap();

        let packages = vec![create_test_package(
            "comment-version",
            package_path.to_str().unwrap(),
            "1.0.0",
        )];

        let setupcfg = SetupCfg::new();
        let result = setupcfg.process_packages(&packages);

        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_path.join("setup.cfg")).unwrap();
        // Should update the real version but leave the commented one alone
        assert!(updated_content.contains("version = 1.0.0"));
        assert!(
            updated_content
                .contains("# version = 0.0.1 (this should not be changed)")
        );
        assert!(!updated_content.contains("version = 0.1.0"));
    }
}
