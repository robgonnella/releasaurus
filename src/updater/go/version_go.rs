use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        generic::updater::{GENERIC_VERSION_REGEX, GenericUpdater},
        manager::UpdaterPackage,
        traits::PackageUpdater,
    },
};

/// Handles version.go file parsing and version updates for Golang packages.
pub struct VersionGo {}

impl VersionGo {
    /// Create VersionGo handler for version.go version updates.
    pub fn new() -> Self {
        Self {}
    }
}

impl PackageUpdater for VersionGo {
    /// Process version.go files for all Golang packages.
    fn update(
        &self,
        package: &UpdaterPackage,
        _workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.basename != "version.go" {
                continue;
            }

            if let Some(change) = GenericUpdater::update_manifest(
                manifest,
                &package.next_version.semver,
                &GENERIC_VERSION_REGEX,
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
    fn updates_const_version() {
        let version_go = VersionGo::new();

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
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Go)),
        };

        let result = version_go.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("const Version = \"2.0.0\""));
    }

    #[test]
    fn updates_var_version() {
        let version_go = VersionGo::new();

        let content = r#"
  var Version = "1.0.0"
"#;
        let manifest = ManifestFile {
            path: Path::new("version.go").to_path_buf(),
            basename: "version.go".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "gopher".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Go)),
        };

        let result = version_go.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("var Version = \"2.0.0\""));
    }

    #[test]
    fn updates_all_caps_version() {
        let version_go = VersionGo::new();

        let content = r#"
  const VERSION = "1.0.0"
"#;
        let manifest = ManifestFile {
            path: Path::new("version.go").to_path_buf(),
            basename: "version.go".to_string(),
            content: content.to_string(),
        };

        let package = UpdaterPackage {
            package_name: "gopher".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Go)),
        };

        let result = version_go.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("const VERSION = \"2.0.0\""));
    }
}
