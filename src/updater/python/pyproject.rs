use log::*;

use toml_edit::{DocumentMut, value};

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
    updater::framework::UpdaterPackage,
};

pub struct PyProject {}

impl PyProject {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn process_packages(
        &self,
        packages: &[UpdaterPackage],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for package in packages {
            let file_path = package.get_file_path("pyproject.toml");

            let doc = self.load_doc(&file_path, loader).await?;

            if doc.is_none() {
                continue;
            }

            info!("found pyproject.toml for package: {}", package.path);

            let mut doc = doc.unwrap();

            if let Some(project) = doc["project"].as_table_mut() {
                if project.get("dynamic").is_some() {
                    info!(
                        "dynamic version found in pyproject.toml: skipping update"
                    );
                    continue;
                }

                info!(
                    "updating {} project version to {}",
                    file_path, package.next_version.semver
                );

                project["version"] =
                    value(package.next_version.semver.to_string());

                file_changes.push(FileChange {
                    path: file_path,
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
                    file_path, package.next_version.semver
                );

                project["version"] =
                    value(package.next_version.semver.to_string());

                file_changes.push(FileChange {
                    path: file_path,
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

    async fn load_doc(
        &self,
        file_path: &str,
        loader: &dyn FileLoader,
    ) -> Result<Option<DocumentMut>> {
        let content = loader.get_file_content(file_path).await?;
        if content.is_none() {
            return Ok(None);
        }
        let content = content.unwrap();
        let doc = content.parse::<DocumentMut>()?;
        Ok(Some(doc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::traits::MockFileLoader;
    use crate::test_helpers::create_test_updater_package;
    use crate::updater::framework::Framework;

    #[tokio::test]
    async fn test_process_packages_project_section() {
        let pyproject = PyProject::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
            Framework::Python,
        );

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

        let packages = vec![package];
        let result = pyproject
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/pyproject.toml");
        assert!(changes[0].content.contains("version = \"2.0.0\""));
        assert!(changes[0].content.contains("name = \"test-package\""));
    }

    #[tokio::test]
    async fn test_process_packages_poetry_section() {
        let pyproject = PyProject::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "3.0.0",
            Framework::Python,
        );

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

        let packages = vec![package];
        let result = pyproject
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/pyproject.toml");
        assert!(changes[0].content.contains("version = \"3.0.0\""));
    }

    #[tokio::test]
    async fn test_process_packages_skips_dynamic_version() {
        let pyproject = PyProject::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
            Framework::Python,
        );

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

        let packages = vec![package];
        let result = pyproject
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        // Should return None because dynamic version is skipped
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_process_packages_preserves_structure() {
        let pyproject = PyProject::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
            Framework::Python,
        );

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

        let packages = vec![package];
        let result = pyproject
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

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

    #[tokio::test]
    async fn test_process_packages_no_file_found() {
        let pyproject = PyProject::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
            Framework::Python,
        );

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/pyproject.toml"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = pyproject
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }
}
