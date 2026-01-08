use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        composite::CompositeUpdater,
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
    composite: CompositeUpdater,
}

impl NodeUpdater {
    /// Create Node.js updater for package.json and lock file management.
    pub fn new() -> Self {
        Self {
            composite: CompositeUpdater::new(vec![
                Box::new(PackageJson::new()),
                Box::new(PackageLock::new()),
                Box::new(YarnLock::new()),
            ]),
        }
    }
}

impl PackageUpdater for NodeUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        self.composite.update(package, workspace_packages)
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, rc::Rc};

    use super::*;
    use crate::{
        analyzer::release::Tag,
        config::release_type::ReleaseType,
        updater::{
            dispatch::Updater,
            manager::{ManifestFile, UpdaterPackage},
        },
    };

    #[test]
    fn processes_node_project() {
        let updater = NodeUpdater::new();
        let content = r#"{"name":"my-package","version":"1.0.0"}"#;
        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Node)),
        };

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_node_files() {
        let updater = NodeUpdater::new();
        let manifest = ManifestFile {
            path: Path::new("Cargo.toml").to_path_buf(),
            basename: "Cargo.toml".to_string(),
            content: "[package]\nversion = \"1.0.0\"\n".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Node)),
        };

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }
}
