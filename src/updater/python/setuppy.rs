use regex::Regex;
use std::sync::LazyLock;

use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::UpdaterPackage,
};

static VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^(\s*version\s*=\s*)([\"'])([\w\.\-\+]+)([\"'])"#)
        .unwrap()
});

pub struct SetupPy {}

impl SetupPy {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename != "setup.py" {
                continue;
            }

            let content = VERSION_REGEX
                .replace(&manifest.content, |caps: &regex::Captures| {
                    format!(
                        "{}{}{}{}",
                        &caps[1],
                        &caps[2],
                        package.next_version.semver,
                        &caps[4]
                    )
                })
                .to_string();

            file_changes.push(FileChange {
                path: manifest.file_path.clone(),
                content,
                update_type: FileUpdateType::Replace,
            });
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
        test_helpers::create_test_tag,
        updater::framework::{Framework, ManifestFile, UpdaterPackage},
    };

    #[tokio::test]
    async fn updates_version_with_double_quotes() {
        let setuppy = SetupPy::new();
        let content =
            "setup(\n    name='my-package',\n    version=\"1.0.0\",\n)\n";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "setup.py".to_string(),
            file_basename: "setup.py".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = setuppy.process_package(&package).await.unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version=\"2.0.0\""));
    }

    #[tokio::test]
    async fn updates_version_with_single_quotes() {
        let setuppy = SetupPy::new();
        let content =
            "setup(\n    name='my-package',\n    version='1.0.0',\n)\n";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "setup.py".to_string(),
            file_basename: "setup.py".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = setuppy.process_package(&package).await.unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version='2.0.0'"));
    }

    #[tokio::test]
    async fn preserves_whitespace_formatting() {
        let setuppy = SetupPy::new();
        let content =
            "setup(\n    name='my-package',\n    version   =   \"1.0.0\",\n)\n";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "setup.py".to_string(),
            file_basename: "setup.py".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = setuppy.process_package(&package).await.unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version   =   \"2.0.0\""));
    }

    #[tokio::test]
    async fn preserves_other_fields() {
        let setuppy = SetupPy::new();
        let content = r#"from setuptools import setup, find_packages

setup(
    name='my-package',
    version="1.0.0",
    description='A test package',
    author='Test Author',
    packages=find_packages(),
    install_requires=[
        'requests>=2.28.0',
    ],
)
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "setup.py".to_string(),
            file_basename: "setup.py".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = setuppy.process_package(&package).await.unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version=\"2.0.0\""));
        assert!(updated.contains("name='my-package'"));
        assert!(updated.contains("description='A test package'"));
        assert!(updated.contains("author='Test Author'"));
        assert!(updated.contains("packages=find_packages()"));
        assert!(updated.contains("'requests>=2.28.0'"));
    }

    #[tokio::test]
    async fn process_package_handles_multiple_setup_py_files() {
        let setuppy = SetupPy::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            file_path: "packages/a/setup.py".to_string(),
            file_basename: "setup.py".to_string(),
            content: "setup(\n    name='package-a',\n    version='1.0.0'\n)"
                .to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            file_path: "packages/b/setup.py".to_string(),
            file_basename: "setup.py".to_string(),
            content: "setup(\n    name='package-b',\n    version='1.0.0'\n)"
                .to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = setuppy.process_package(&package).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[tokio::test]
    async fn process_package_returns_none_when_no_setup_py_files() {
        let setuppy = SetupPy::new();
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "setup.cfg".to_string(),
            file_basename: "setup.cfg".to_string(),
            content: "[metadata]\nversion = 1.0.0\n".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = setuppy.process_package(&package).await.unwrap();

        assert!(result.is_none());
    }
}
