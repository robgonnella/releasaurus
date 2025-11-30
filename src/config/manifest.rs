use color_eyre::eyre::OptionExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{
    Result,
    config::{
        manifest::{
            java::JavaManifestLoader, node::NodeManifestLoader,
            php::PhpManifestLoader, python::PythonManifestLoader,
            ruby::RubyManifestLoader, rust::RustManifestLoader,
        },
        package::PackageConfig,
        release_type::ReleaseType,
    },
    forge::manager::ForgeManager,
};

mod java;
mod node;
mod php;
mod python;
mod ruby;
mod rust;

#[derive(
    Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
pub struct ManifestFile {
    #[serde(skip)]
    /// Whether or not to treat this as a workspace manifest
    pub is_workspace: bool,
    #[serde(rename = "path")]
    /// The file path relative to the package path that will be updated using a
    /// generic regex version replace
    pub file_path: String,
    #[serde(skip)]
    /// The base name of the file path
    pub file_basename: String,
    #[serde(skip)]
    /// The current content of the file
    pub content: String,
}

fn gen_package_path(package: &PackageConfig, file: &str) -> String {
    Path::new(&package.workspace_root)
        .join(&package.path)
        .join(file)
        .display()
        .to_string()
        .replace("./", "")
}

fn gen_workspace_path(package: &PackageConfig, file: &str) -> String {
    Path::new(&package.workspace_root)
        .join(file)
        .display()
        .to_string()
        .replace("./", "")
}

pub async fn load_release_type_manifests_for_package(
    forge: &ForgeManager,
    pkg: &PackageConfig,
) -> Result<Option<Vec<ManifestFile>>> {
    match pkg.release_type.clone() {
        Some(ReleaseType::Generic) => Ok(None),
        Some(ReleaseType::Java) => {
            JavaManifestLoader::load_manifests(forge, pkg).await
        }
        Some(ReleaseType::Node) => {
            NodeManifestLoader::load_manifests(forge, pkg).await
        }
        Some(ReleaseType::Php) => {
            PhpManifestLoader::load_manifests(forge, pkg).await
        }
        Some(ReleaseType::Python) => {
            PythonManifestLoader::load_manifests(forge, pkg).await
        }
        Some(ReleaseType::Ruby) => {
            RubyManifestLoader::load_manifests(forge, pkg).await
        }
        Some(ReleaseType::Rust) => {
            RustManifestLoader::load_manifests(forge, pkg).await
        }
        None => Ok(None),
    }
}

pub async fn load_additional_manifests_for_package(
    forge: &ForgeManager,
    pkg: &PackageConfig,
) -> Result<Option<Vec<ManifestFile>>> {
    if let Some(additional) = pkg.additional_manifest_files.clone() {
        let mut manifests = vec![];

        for extra in additional {
            let full_path = gen_package_path(pkg, &extra.file_path);

            let basename = Path::new(&full_path)
                .file_name()
                .ok_or_eyre(format!(
                    "unable to determine manifest basename from path: {}",
                    full_path
                ))?
                .display()
                .to_string();

            if let Some(content) = forge.get_file_content(&full_path).await? {
                manifests.push(ManifestFile {
                    content,
                    file_basename: basename,
                    file_path: full_path,
                    is_workspace: false,
                });
            }
        }

        if manifests.is_empty() {
            return Ok(None);
        }

        return Ok(Some(manifests));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::release_type::ReleaseType, forge::traits::MockForge,
        test_helpers::create_test_remote_config,
    };

    // ===== Test Helpers =====

    fn package_config(
        name: &str,
        path: &str,
        workspace_root: &str,
    ) -> PackageConfig {
        PackageConfig {
            name: name.to_string(),
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

    // ===== Path Generation Tests =====

    #[test]
    fn generates_package_path_at_root() {
        let pkg = package_config("pkg", ".", ".");
        assert_eq!(gen_package_path(&pkg, "file.txt"), "file.txt");
    }

    #[test]
    fn generates_package_path_in_subdirectory() {
        let pkg = package_config("pkg", "packages/my-pkg", ".");
        assert_eq!(
            gen_package_path(&pkg, "file.txt"),
            "packages/my-pkg/file.txt"
        );
    }

    #[test]
    fn generates_package_path_with_workspace_root() {
        let pkg = package_config("pkg", "src", "workspace");
        assert_eq!(
            gen_package_path(&pkg, "file.txt"),
            "workspace/src/file.txt"
        );
    }

    #[test]
    fn generates_workspace_path() {
        let pkg = package_config("pkg", "packages/my-pkg", ".");
        assert_eq!(gen_workspace_path(&pkg, "file.txt"), "file.txt");
    }

    #[test]
    fn generates_workspace_path_with_workspace_root() {
        let pkg = package_config("pkg", "src", "workspace");
        assert_eq!(gen_workspace_path(&pkg, "file.txt"), "workspace/file.txt");
    }

    // ===== load_release_type_manifests_for_package Tests =====

    #[tokio::test]
    async fn returns_none_for_generic_release_type() {
        let mut pkg = package_config("pkg", ".", ".");
        pkg.release_type = Some(ReleaseType::Generic);
        let forge = mock_forge_empty();

        let result = load_release_type_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn returns_none_when_no_release_type() {
        let pkg = package_config("pkg", ".", ".");
        let forge = mock_forge_empty();

        let result = load_release_type_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn loads_node_manifests() {
        let mut pkg = package_config("pkg", ".", ".");
        pkg.release_type = Some(ReleaseType::Node);
        let forge = mock_forge_with_file("package.json", "{}");

        let result = load_release_type_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].file_basename, "package.json");
    }

    #[tokio::test]
    async fn loads_rust_manifests() {
        let mut pkg = package_config("pkg", ".", ".");
        pkg.release_type = Some(ReleaseType::Rust);
        let forge = mock_forge_with_file("Cargo.toml", "[package]");

        let result = load_release_type_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].file_basename, "Cargo.toml");
    }

    #[tokio::test]
    async fn loads_python_manifests() {
        let mut pkg = package_config("pkg", ".", ".");
        pkg.release_type = Some(ReleaseType::Python);
        let forge = mock_forge_with_file("pyproject.toml", "[project]");

        let result = load_release_type_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].file_basename, "pyproject.toml");
    }

    #[tokio::test]
    async fn loads_php_manifests() {
        let mut pkg = package_config("pkg", ".", ".");
        pkg.release_type = Some(ReleaseType::Php);
        let forge = mock_forge_with_file("composer.json", "{}");

        let result = load_release_type_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].file_basename, "composer.json");
    }

    #[tokio::test]
    async fn loads_java_manifests() {
        let mut pkg = package_config("pkg", ".", ".");
        pkg.release_type = Some(ReleaseType::Java);
        let forge = mock_forge_with_file("pom.xml", "<project>");

        let result = load_release_type_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].file_basename, "pom.xml");
    }

    #[tokio::test]
    async fn loads_ruby_manifests() {
        let mut pkg = package_config("my-gem", ".", ".");
        pkg.release_type = Some(ReleaseType::Ruby);
        let forge =
            mock_forge_with_file("my-gem.gemspec", "Gem::Specification");

        let result = load_release_type_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].file_basename, "my-gem.gemspec");
    }

    #[tokio::test]
    async fn returns_none_when_no_manifests_found() {
        let mut pkg = package_config("pkg", ".", ".");
        pkg.release_type = Some(ReleaseType::Node);
        let forge = mock_forge_empty();

        let result = load_release_type_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    // ===== load_additional_manifests_for_package Tests =====

    #[tokio::test]
    async fn returns_none_when_no_additional_manifests_configured() {
        let pkg = package_config("pkg", ".", ".");
        let forge = mock_forge_empty();

        let result = load_additional_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn loads_additional_manifest_file() {
        let mut pkg = package_config("pkg", ".", ".");
        pkg.additional_manifest_files = Some(vec![ManifestFile {
            is_workspace: false,
            file_path: "VERSION".to_string(),
            file_basename: "VERSION".to_string(),
            content: String::new(),
        }]);
        let forge = mock_forge_with_file("VERSION", "1.0.0");

        let result = load_additional_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].file_basename, "VERSION");
        assert_eq!(manifests[0].content, "1.0.0");
    }

    #[tokio::test]
    async fn loads_multiple_additional_manifests() {
        let mut pkg = package_config("pkg", ".", ".");
        pkg.additional_manifest_files = Some(vec![
            ManifestFile {
                is_workspace: false,
                file_path: "VERSION".to_string(),
                file_basename: "VERSION".to_string(),
                content: String::new(),
            },
            ManifestFile {
                is_workspace: false,
                file_path: "docs/VERSION.txt".to_string(),
                file_basename: "VERSION.txt".to_string(),
                content: String::new(),
            },
        ]);

        let mut mock = MockForge::new();
        mock.expect_get_file_content().returning(|path| {
            if path == "VERSION" || path == "docs/VERSION.txt" {
                Ok(Some("1.0.0".to_string()))
            } else {
                Ok(None)
            }
        });
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        let forge = ForgeManager::new(Box::new(mock));

        let result = load_additional_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 2);
    }

    #[tokio::test]
    async fn returns_none_when_additional_manifests_not_found() {
        let mut pkg = package_config("pkg", ".", ".");
        pkg.additional_manifest_files = Some(vec![ManifestFile {
            is_workspace: false,
            file_path: "VERSION".to_string(),
            file_basename: "VERSION".to_string(),
            content: String::new(),
        }]);
        let forge = mock_forge_empty();

        let result = load_additional_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn generates_correct_path_for_additional_manifests() {
        let mut pkg = package_config("pkg", "packages/my-pkg", ".");
        pkg.additional_manifest_files = Some(vec![ManifestFile {
            is_workspace: false,
            file_path: "VERSION".to_string(),
            file_basename: "VERSION".to_string(),
            content: String::new(),
        }]);
        let forge = mock_forge_with_file("packages/my-pkg/VERSION", "1.0.0");

        let result = load_additional_manifests_for_package(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_path, "packages/my-pkg/VERSION");
    }
}
