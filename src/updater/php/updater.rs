use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        manager::UpdaterPackage, php::composer_json::ComposerJson,
        traits::PackageUpdater,
    },
};

/// PHP package updater for Composer projects.
pub struct PhpUpdater {
    composer_json: ComposerJson,
}

impl PhpUpdater {
    /// Create PHP updater for Composer composer.json files.
    pub fn new() -> Self {
        Self {
            composer_json: ComposerJson::new(),
        }
    }
}

impl PackageUpdater for PhpUpdater {
    fn update(
        &self,
        package: &UpdaterPackage,
        // workspaces not supported for php projects
        _workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        self.composer_json.process_package(package)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyzer::release::Tag,
        config::release_type::ReleaseType,
        updater::manager::{ManifestFile, UpdaterPackage},
    };

    #[test]
    fn processes_php_project() {
        let updater = PhpUpdater::new();
        let content = r#"{"name":"vendor/package","version":"1.0.0"}"#;
        let manifest = ManifestFile {
            is_workspace: false,
            path: "composer.json".to_string(),
            basename: "composer.json".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "vendor/package".to_string(),
            manifest_files: vec![manifest],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            release_type: ReleaseType::Php,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_some());
        assert!(result.unwrap()[0].content.contains("2.0.0"));
    }

    #[test]
    fn returns_none_when_no_php_files() {
        let updater = PhpUpdater::new();
        let manifest = ManifestFile {
            is_workspace: false,
            path: "package.json".to_string(),
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
            release_type: ReleaseType::Php,
        };

        let result = updater.update(&package, vec![]).unwrap();

        assert!(result.is_none());
    }
}
