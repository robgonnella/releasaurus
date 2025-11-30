use crate::{
    Result,
    config::{
        manifest::{ManifestFile, gen_package_path},
        package::PackageConfig,
    },
    forge::manager::ForgeManager,
};

pub struct RubyManifestLoader {}

struct Target {
    path: String,
    basename: String,
}

impl RubyManifestLoader {
    pub async fn load_manifests(
        forge: &ForgeManager,
        pkg: &PackageConfig,
    ) -> Result<Option<Vec<ManifestFile>>> {
        let pkg_gemspec = format!("{}.gemspec", pkg.name);
        let lib_pkg_version = format!("lib/{}/version.rb", pkg.name);

        let targets = vec![
            Target {
                path: pkg_gemspec.clone(),
                basename: pkg_gemspec,
            },
            Target {
                path: lib_pkg_version,
                basename: "version.rb".into(),
            },
            Target {
                path: "lib/version.rb".into(),
                basename: "version.rb".into(),
            },
            Target {
                path: "version.rb".into(),
                basename: "version.rb".into(),
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

    // ===== Manifest Loading Tests =====

    #[tokio::test]
    async fn loads_gemspec_file() {
        let pkg = package_config("my-gem", ".", ".");
        let forge = mock_forge_with_file(
            "my-gem.gemspec",
            "Gem::Specification.new do |spec|\n  spec.version = '1.0.0'\nend",
        );

        let result = RubyManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].file_basename, "my-gem.gemspec");
        assert!(!manifests[0].is_workspace);
    }

    #[tokio::test]
    async fn loads_version_rb_in_lib_package_name() {
        let pkg = package_config("my-gem", ".", ".");
        let forge = mock_forge_with_file(
            "lib/my-gem/version.rb",
            "module MyGem\n  VERSION = '1.0.0'\nend",
        );

        let result = RubyManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_basename, "version.rb");
        assert_eq!(manifests[0].file_path, "lib/my-gem/version.rb");
    }

    #[tokio::test]
    async fn loads_version_rb_in_lib() {
        let pkg = package_config("my-gem", ".", ".");
        let forge = mock_forge_with_file("lib/version.rb", "VERSION = '1.0.0'");

        let result = RubyManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_basename, "version.rb");
        assert_eq!(manifests[0].file_path, "lib/version.rb");
    }

    #[tokio::test]
    async fn loads_version_rb_at_root() {
        let pkg = package_config("my-gem", ".", ".");
        let forge = mock_forge_with_file("version.rb", "VERSION = '1.0.0'");

        let result = RubyManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_basename, "version.rb");
        assert_eq!(manifests[0].file_path, "version.rb");
    }

    #[tokio::test]
    async fn loads_multiple_manifests() {
        let pkg = package_config("my-gem", ".", ".");
        let mut mock = MockForge::new();
        mock.expect_get_file_content().returning(|path| {
            if path == "my-gem.gemspec" {
                Ok(Some("spec.version = '1.0.0'".to_string()))
            } else if path == "lib/my-gem/version.rb" {
                Ok(Some("VERSION = '1.0.0'".to_string()))
            } else {
                Ok(None)
            }
        });
        mock.expect_remote_config()
            .returning(create_test_remote_config);
        let forge = ForgeManager::new(Box::new(mock));

        let result = RubyManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests.len(), 2);
    }

    #[tokio::test]
    async fn returns_none_when_no_manifests_found() {
        let pkg = package_config("my-gem", ".", ".");
        let forge = mock_forge_empty();

        let result = RubyManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn uses_correct_path_for_subpackage() {
        let pkg = package_config("my-gem", "packages/my-gem", ".");
        let forge = mock_forge_with_file(
            "packages/my-gem/my-gem.gemspec",
            "spec.version = '1.0.0'",
        );

        let result = RubyManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_path, "packages/my-gem/my-gem.gemspec");
    }

    #[tokio::test]
    async fn uses_correct_path_with_workspace_root() {
        let pkg = package_config("my-gem", "src", "workspace");
        let forge =
            mock_forge_with_file("workspace/src/my-gem.gemspec", "spec");

        let result = RubyManifestLoader::load_manifests(&forge, &pkg)
            .await
            .unwrap();

        assert!(result.is_some());
        let manifests = result.unwrap();
        assert_eq!(manifests[0].file_path, "workspace/src/my-gem.gemspec");
    }
}
