use crate::{
    cli::Result,
    forge::request::FileChange,
    updater::{framework::UpdaterPackage, generic::updater::GenericUpdater},
};

pub struct SetupCfg {}

impl SetupCfg {
    pub fn new() -> Self {
        Self {}
    }

    pub fn process_package(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename != "setup.cfg" {
                continue;
            }

            if let Some(change) = GenericUpdater::update_manifest(
                manifest,
                &package.next_version.semver,
            ) {
                file_changes.push(change);
            }
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
        config::ManifestFile,
        test_helpers::create_test_tag,
        updater::framework::{Framework, UpdaterPackage},
    };

    #[tokio::test]
    async fn updates_version_without_quotes() {
        let setupcfg = SetupCfg::new();
        let content = "[metadata]\nname = my-package\nversion = 1.0.0\n";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "setup.cfg".to_string(),
            file_basename: "setup.cfg".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = setupcfg.process_package(&package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = 2.0.0"));
    }

    #[tokio::test]
    async fn updates_version_with_double_quotes() {
        let setupcfg = SetupCfg::new();
        let content = "[metadata]\nname = my-package\nversion = \"1.0.0\"\n";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "setup.cfg".to_string(),
            file_basename: "setup.cfg".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = setupcfg.process_package(&package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
    }

    #[tokio::test]
    async fn updates_version_with_single_quotes() {
        let setupcfg = SetupCfg::new();
        let content = "[metadata]\nname = my-package\nversion = '1.0.0'\n";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "setup.cfg".to_string(),
            file_basename: "setup.cfg".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = setupcfg.process_package(&package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = '2.0.0'"));
    }

    #[tokio::test]
    async fn preserves_whitespace_formatting() {
        let setupcfg = SetupCfg::new();
        let content = "[metadata]\nname = my-package\nversion   =   1.0.0\n";
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "setup.cfg".to_string(),
            file_basename: "setup.cfg".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = setupcfg.process_package(&package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version   =   2.0.0"));
    }

    #[tokio::test]
    async fn preserves_other_fields() {
        let setupcfg = SetupCfg::new();
        let content = r#"[metadata]
name = my-package
version = 1.0.0
description = A test package
author = Test Author

[options]
packages = find:
install_requires =
    requests>=2.28.0
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "setup.cfg".to_string(),
            file_basename: "setup.cfg".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = setupcfg.process_package(&package).unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version = 2.0.0"));
        assert!(updated.contains("name = my-package"));
        assert!(updated.contains("description = A test package"));
        assert!(updated.contains("author = Test Author"));
        assert!(updated.contains("packages = find:"));
        assert!(updated.contains("requests>=2.28.0"));
    }

    #[tokio::test]
    async fn process_package_handles_multiple_setup_cfg_files() {
        let setupcfg = SetupCfg::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            file_path: "packages/a/setup.cfg".to_string(),
            file_basename: "setup.cfg".to_string(),
            content: "[metadata]\nversion = 1.0.0\n".to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            file_path: "packages/b/setup.cfg".to_string(),
            file_basename: "setup.cfg".to_string(),
            content: "[metadata]\nversion = 1.0.0\n".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = setupcfg.process_package(&package).unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[tokio::test]
    async fn process_package_returns_none_when_no_setup_cfg_files() {
        let setupcfg = SetupCfg::new();
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "setup.py".to_string(),
            file_basename: "setup.py".to_string(),
            content: "setup(name='my-package', version='1.0.0')".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Python,
        };

        let result = setupcfg.process_package(&package).unwrap();

        assert!(result.is_none());
    }
}
