use crate::{
    Result,
    config::{
        manifest::{ManifestFile, gen_package_path},
        package::PackageConfig,
    },
    forge::manager::ForgeManager,
};

pub struct JavaManifestLoader {}

struct Target {
    path: String,
    basename: String,
}

impl JavaManifestLoader {
    pub async fn load_manifests(
        forge: &ForgeManager,
        pkg: &PackageConfig,
    ) -> Result<Option<Vec<ManifestFile>>> {
        let targets = vec![
            Target {
                path: "build.gradle".into(),
                basename: "build.gradle".into(),
            },
            Target {
                path: "lib/build.gradle".into(),
                basename: "build.gradle".into(),
            },
            Target {
                path: "build.gradle.kts".into(),
                basename: "build.gradle.kts".into(),
            },
            Target {
                path: "lib/build.gradle.kts".into(),
                basename: "build.gradle.kts".into(),
            },
            Target {
                path: "gradle.properties".into(),
                basename: "gradle.properties".into(),
            },
            Target {
                path: "pom.xml".into(),
                basename: "pom.xml".into(),
            },
        ];

        let mut manifests = vec![];

        for target in targets {
            let full_path = gen_package_path(pkg, &target.path);

            if let Some(content) = forge.get_file_content(&full_path).await? {
                manifests.push(ManifestFile {
                    file_path: full_path,
                    file_basename: target.basename,
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
            name: "my-project".to_string(),
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
    async fn loads_gradle_build_file() {
        let pkg = package_config(".", ".");
        let forge = mock_forge_with_file("build.gradle", "version = '1.0.0'");

        let result = JavaManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].file_basename, "build.gradle");
        assert!(manifests[0].content.contains("1.0.0"));
    }

    #[tokio::test]
    async fn loads_kotlin_gradle_build_file() {
        let pkg = package_config(".", ".");
        let forge =
            mock_forge_with_file("build.gradle.kts", "version = \"1.0.0\"");

        let result = JavaManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_basename, "build.gradle.kts");
    }

    #[tokio::test]
    async fn loads_gradle_properties() {
        let pkg = package_config(".", ".");
        let forge = mock_forge_with_file("gradle.properties", "version=1.0.0");

        let result = JavaManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_basename, "gradle.properties");
    }

    #[tokio::test]
    async fn loads_maven_pom() {
        let pkg = package_config(".", ".");
        let forge = mock_forge_with_file("pom.xml", "<version>1.0.0</version>");

        let result = JavaManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_basename, "pom.xml");
    }

    #[tokio::test]
    async fn loads_lib_gradle_build_file() {
        let pkg = package_config(".", ".");
        let forge =
            mock_forge_with_file("lib/build.gradle", "version = '1.0.0'");

        let result = JavaManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_basename, "build.gradle");
        assert_eq!(manifests[0].file_path, "lib/build.gradle");
    }

    #[tokio::test]
    async fn loads_multiple_manifests() {
        let pkg = package_config(".", ".");
        let mut mock = MockForge::new();
        mock.expect_get_file_content().returning(|path| {
            if path == "build.gradle" {
                Ok(Some("version = '1.0.0'".to_string()))
            } else if path == "gradle.properties" {
                Ok(Some("version=1.0.0".to_string()))
            } else {
                Ok(None)
            }
        });
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        let forge = ForgeManager::new(Box::new(mock));

        let result = JavaManifestLoader::load_manifests(&forge, &pkg)
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

        let result = JavaManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn uses_correct_path_for_subpackage() {
        let pkg = package_config("packages/my-java-lib", ".");
        let forge =
            mock_forge_with_file("packages/my-java-lib/pom.xml", "<project>");

        let result = JavaManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_path, "packages/my-java-lib/pom.xml");
    }
}
