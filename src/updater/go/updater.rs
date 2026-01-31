use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        go::version_go::VersionGo, manager::UpdaterPackage,
        traits::PackageUpdater,
    },
};

/// Golang package updater
pub struct GoUpdater {
    version_go: VersionGo,
}

impl GoUpdater {
    /// Create Golang updater.
    pub fn new() -> Self {
        Self {
            version_go: VersionGo::new(),
        }
    }
}

impl PackageUpdater for GoUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        self.version_go.update(package, workspace_packages)
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
    fn processes_go_project() {
        let updater = GoUpdater::new();
        let content = r#"
const Version = "1.0.0"
"#;
        let manifest = ManifestFile {
            path: Path::new("version.go").to_path_buf(),
            basename: "version.go".to_string(),
            content: content.to_string(),
        };

        let package = UpdaterPackage {
            package_name: "gopher".to_string(),
            manifest_files: vec![manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Go)),
        };

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_go_files() {
        let updater = GoUpdater::new();
        let manifest = ManifestFile {
            path: Path::new("package.json").to_path_buf(),
            basename: "package.json".to_string(),
            content: r#"{"version":"1.0.0"}"#.to_string(),
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
            updater: Rc::new(Updater::new(ReleaseType::Go)),
        };

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }
}
