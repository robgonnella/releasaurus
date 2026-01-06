use crate::{
    Result,
    forge::request::FileChange,
    updater::{
        generic::updater::{GENERIC_VERSION_REGEX, GenericUpdater},
        manager::UpdaterPackage,
        traits::PackageUpdater,
    },
};

pub struct SetupPy {}

impl SetupPy {
    pub fn new() -> Self {
        Self {}
    }
}

impl PackageUpdater for SetupPy {
    fn update(
        &self,
        package: &UpdaterPackage,
        _workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.basename != "setup.py" {
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
    use std::rc::Rc;

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
    fn updates_version_with_double_quotes() {
        let setuppy = SetupPy::new();
        let content =
            "setup(\n    name='my-package',\n    version=\"1.0.0\",\n)\n";
        let manifest = ManifestFile {
            path: "setup.py".to_string(),
            basename: "setup.py".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = setuppy.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version=\"2.0.0\""));
    }

    #[test]
    fn updates_version_with_single_quotes() {
        let setuppy = SetupPy::new();
        let content =
            "setup(\n    name='my-package',\n    version='1.0.0',\n)\n";
        let manifest = ManifestFile {
            path: "setup.py".to_string(),
            basename: "setup.py".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = setuppy.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version='2.0.0'"));
    }

    #[test]
    fn preserves_whitespace_formatting() {
        let setuppy = SetupPy::new();
        let content =
            "setup(\n    name='my-package',\n    version   =   \"1.0.0\",\n)\n";
        let manifest = ManifestFile {
            path: "setup.py".to_string(),
            basename: "setup.py".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = setuppy.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version   =   \"2.0.0\""));
    }

    #[test]
    fn preserves_other_fields() {
        let setuppy = SetupPy::new();
        let content = r#"from setuptools import setup, find_packages

setup(
    name='my-package',
    version="1.0.0",
    description='A test package',
    author='Test Author',
    packages=find_packages(),
    install_requires=[
        'requests>=2.28.0',
    ],
)
"#;
        let manifest = ManifestFile {
            path: "setup.py".to_string(),
            basename: "setup.py".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = setuppy.update(&package, &[]).unwrap();

        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version=\"2.0.0\""));
        assert!(updated.contains("name='my-package'"));
        assert!(updated.contains("description='A test package'"));
        assert!(updated.contains("author='Test Author'"));
        assert!(updated.contains("packages=find_packages()"));
        assert!(updated.contains("'requests>=2.28.0'"));
    }

    #[test]
    fn process_package_handles_multiple_setup_py_files() {
        let setuppy = SetupPy::new();
        let manifest1 = ManifestFile {
            path: "packages/a/setup.py".to_string(),
            basename: "setup.py".to_string(),
            content: "setup(\n    name='package-a',\n    version='1.0.0'\n)"
                .to_string(),
        };
        let manifest2 = ManifestFile {
            path: "packages/b/setup.py".to_string(),
            basename: "setup.py".to_string(),
            content: "setup(\n    name='package-b',\n    version='1.0.0'\n)"
                .to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: Tag {
                name: "v2.0.0".into(),
                semver: semver::Version::parse("2.0.0").unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = setuppy.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[test]
    fn process_package_returns_none_when_no_setup_py_files() {
        let setuppy = SetupPy::new();
        let manifest = ManifestFile {
            path: "setup.cfg".to_string(),
            basename: "setup.cfg".to_string(),
            content: "[metadata]\nversion = 1.0.0\n".to_string(),
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
            updater: Rc::new(Updater::new(ReleaseType::Python)),
        };

        let result = setuppy.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }
}
