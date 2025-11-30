use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        manager::UpdaterPackage,
        node::{
            package_json::PackageJson, package_lock::PackageLock,
            yarn_lock::YarnLock,
        },
        traits::PackageUpdater,
    },
};

/// Node.js package updater for npm, yarn, and pnpm projects.
pub struct NodeUpdater {
    package_json: PackageJson,
    package_lock: PackageLock,
    yarn_lock: YarnLock,
}

impl NodeUpdater {
    /// Create Node.js updater for package.json and lock file management.
    pub fn new() -> Self {
        Self {
            package_json: PackageJson::new(),
            package_lock: PackageLock::new(),
            yarn_lock: YarnLock::new(),
        }
    }
}

impl PackageUpdater for NodeUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        if let Some(changes) = self
            .package_json
            .process_package(package, &workspace_packages)?
        {
            file_changes.extend(changes);
        }

        if let Some(changes) = self
            .package_lock
            .process_package(package, &workspace_packages)?
        {
            file_changes.extend(changes);
        }

        if let Some(changes) = self
            .yarn_lock
            .process_package(package, &workspace_packages)?
        {
            file_changes.extend(changes);
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
        config::release_type::ReleaseType,
        test_helpers::create_test_tag,
        updater::manager::{ManifestFile, UpdaterPackage},
    };

    #[test]
    fn processes_node_project() {
        let updater = NodeUpdater::new();
        let content = r#"{"name":"my-package","version":"1.0.0"}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "package.json".to_string(),
            basename: "package.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            release_type: ReleaseType::Node,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_some());
        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_node_files() {
        let updater = NodeUpdater::new();
        let manifest = ManifestFile {
            is_workspace: false,
            path: "Cargo.toml".to_string(),
            basename: "Cargo.toml".to_string(),
            content: "[package]\nversion = \"1.0.0\"\n".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            release_type: ReleaseType::Node,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_none());
    }
}
