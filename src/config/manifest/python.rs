use crate::{
    Result,
    config::{
        manifest::{ManifestFile, gen_package_path},
        package::PackageConfig,
    },
    forge::manager::ForgeManager,
};

pub struct PythonManifestLoader {}

impl PythonManifestLoader {
    pub async fn load_manifests(
        forge: &ForgeManager,
        pkg: &PackageConfig,
    ) -> Result<Option<Vec<ManifestFile>>> {
        let files = vec!["pyproject.toml", "setup.cfg", "setup.py"];
        let mut manifests = vec![];

        for file in files {
            let full_path = gen_package_path(pkg, file);
            if let Some(content) = forge.get_file_content(&full_path).await? {
                manifests.push(ManifestFile {
                    file_path: full_path,
                    file_basename: file.to_string(),
                    is_workspace: false,
                    content,
                });
            }
        }

        if manifests.is_empty() {
            return Ok(None);
        }

        Ok(Some(manifests))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        forge::traits::MockForge, test_helpers::create_test_remote_config,
    };

    // ===== Test Helpers =====

    fn package_config(path: &str, workspace_root: &str) -> PackageConfig {
        PackageConfig {
            name: "my-package".to_string(),
            path: path.to_string(),
            workspace_root: workspace_root.to_string(),
            release_type: None,
            tag_prefix: None,
            prerelease: None,
            additional_paths: None,
            additional_manifest_files: None,
            breaking_always_increment_major: None,
            features_always_increment_minor: None,
            custom_major_increment_regex: None,
            custom_minor_increment_regex: None,
        }
    }

    fn mock_forge_with_file(path: &str, content: &str) -> ForgeManager {
        let mut mock = MockForge::new();
        let path = path.to_string();
        let content = content.to_string();
        mock.expect_get_file_content().returning(move |p| {
            if p == path {
                Ok(Some(content.clone()))
            } else {
                Ok(None)
            }
        });
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        ForgeManager::new(Box::new(mock))
    }

    fn mock_forge_empty() -> ForgeManager {
        let mut mock = MockForge::new();
        mock.expect_get_file_content().returning(|_| Ok(None));
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        ForgeManager::new(Box::new(mock))
    }

    // ===== Manifest Loading Tests =====

    #[tokio::test]
    async fn loads_pyproject_toml() {
        let pkg = package_config(".", ".");
        let forge = mock_forge_with_file(
            "pyproject.toml",
            r#"[project]
name = "my-package"
version = "1.0.0""#,
        );

        let result = PythonManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].file_basename, "pyproject.toml");
        assert!(!manifests[0].is_workspace);
    }

    #[tokio::test]
    async fn loads_setup_cfg() {
        let pkg = package_config(".", ".");
        let forge = mock_forge_with_file(
            "setup.cfg",
            "[metadata]\nname = my-package\nversion = 1.0.0",
        );

        let result = PythonManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_basename, "setup.cfg");
    }

    #[tokio::test]
    async fn loads_setup_py() {
        let pkg = package_config(".", ".");
        let forge = mock_forge_with_file(
            "setup.py",
            "from setuptools import setup\nsetup(version='1.0.0')",
        );

        let result = PythonManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_basename, "setup.py");
    }

    #[tokio::test]
    async fn loads_multiple_manifests() {
        let pkg = package_config(".", ".");
        let mut mock = MockForge::new();
        mock.expect_get_file_content().returning(|path| {
            if path == "pyproject.toml" {
                Ok(Some("[project]\nversion = \"1.0.0\"".to_string()))
            } else if path == "setup.py" {
                Ok(Some("setup(version='1.0.0')".to_string()))
            } else {
                Ok(None)
            }
        });
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        let forge = ForgeManager::new(Box::new(mock));

        let result = PythonManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 2);
    }

    #[tokio::test]
    async fn returns_none_when_no_manifests_found() {
        let pkg = package_config(".", ".");
        let forge = mock_forge_empty();

        let result = PythonManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn uses_correct_path_for_subpackage() {
        let pkg = package_config("packages/my-python-lib", ".");
        let forge = mock_forge_with_file(
            "packages/my-python-lib/pyproject.toml",
            "[project]",
        );

        let result = PythonManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(
            manifests[0].file_path,
            "packages/my-python-lib/pyproject.toml"
        );
    }

    #[tokio::test]
    async fn uses_correct_path_with_workspace_root() {
        let pkg = package_config("src", "workspace");
        let forge =
            mock_forge_with_file("workspace/src/pyproject.toml", "[project]");

        let result = PythonManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_path, "workspace/src/pyproject.toml");
    }
}
