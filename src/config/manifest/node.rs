use crate::{
    Result,
    config::{
        manifest::{ManifestFile, gen_package_path, gen_workspace_path},
        package::PackageConfig,
    },
    forge::manager::ForgeManager,
};

pub struct NodeManifestLoader {}

impl NodeManifestLoader {
    pub async fn load_manifests(
        forge: &ForgeManager,
        pkg: &PackageConfig,
    ) -> Result<Option<Vec<ManifestFile>>> {
        let package_json_pkg_path = gen_package_path(pkg, "package.json");
        let package_json_wrkspc_path = gen_workspace_path(pkg, "package.json");
        let is_workspace_pkg =
            package_json_pkg_path == package_json_wrkspc_path;

        let package_files =
            vec!["package.json", "package-lock.json", "yarn.lock"];

        let workspace_files = ["package-lock.json", "yarn.lock"];

        let mut manifests = vec![];

        for file in package_files {
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

        if is_workspace_pkg {
            for file in workspace_files {
                let full_path = gen_workspace_path(pkg, file);
                if let Some(content) =
                    forge.get_file_content(&full_path).await?
                {
                    manifests.push(ManifestFile {
                        file_path: full_path,
                        file_basename: file.to_string(),
                        is_workspace: true,
                        content,
                    });
                }
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
    async fn loads_package_json() {
        let pkg = package_config(".", ".");
        let forge =
            mock_forge_with_file("package.json", r#"{"version":"1.0.0"}"#);

        let result = NodeManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].file_basename, "package.json");
        assert!(!manifests[0].is_workspace);
    }

    #[tokio::test]
    async fn loads_package_lock_json() {
        let pkg = package_config(".", ".");
        let forge =
            mock_forge_with_file("package-lock.json", r#"{"version":"1.0.0"}"#);

        let result = NodeManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_basename, "package-lock.json");
    }

    #[tokio::test]
    async fn loads_yarn_lock() {
        let pkg = package_config(".", ".");
        let forge = mock_forge_with_file("yarn.lock", "# yarn lockfile v1");

        let result = NodeManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_basename, "yarn.lock");
    }

    #[tokio::test]
    async fn loads_multiple_manifests_in_subpackage() {
        let pkg = package_config("packages/my-pkg", ".");
        let mut mock = MockForge::new();
        mock.expect_get_file_content().returning(|path| {
            if path == "packages/my-pkg/package.json"
                || path == "packages/my-pkg/package-lock.json"
            {
                Ok(Some(r#"{"version":"1.0.0"}"#.to_string()))
            } else {
                Ok(None)
            }
        });
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        let forge = ForgeManager::new(Box::new(mock));

        let result = NodeManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 2);
    }

    #[tokio::test]
    async fn loads_workspace_files_for_root_package() {
        let pkg = package_config(".", ".");
        let mut mock = MockForge::new();
        mock.expect_get_file_content().returning(|path| {
            if path == "package.json" || path == "package-lock.json" {
                Ok(Some(r#"{"version":"1.0.0"}"#.to_string()))
            } else {
                Ok(None)
            }
        });
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        let forge = ForgeManager::new(Box::new(mock));

        let result = NodeManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        // Should have both package files, with workspace lock marked
        assert!(!manifests.is_empty());
        let workspace_locks =
            manifests.iter().filter(|m| m.is_workspace).count();
        assert_eq!(workspace_locks, 1);
    }

    #[tokio::test]
    async fn does_not_load_workspace_files_for_subpackage() {
        let pkg = package_config("packages/my-pkg", ".");
        let forge = mock_forge_with_file("packages/my-pkg/package.json", "{}");

        let result = NodeManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        // All manifests should be non-workspace
        assert!(manifests.iter().all(|m| !m.is_workspace));
    }

    #[tokio::test]
    async fn returns_none_when_no_manifests_found() {
        let pkg = package_config(".", ".");
        let forge = mock_forge_empty();

        let result = NodeManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn uses_correct_path_for_subpackage() {
        let pkg = package_config("packages/my-node-lib", ".");
        let forge =
            mock_forge_with_file("packages/my-node-lib/package.json", "{}");

        let result = NodeManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_path, "packages/my-node-lib/package.json");
    }
}
