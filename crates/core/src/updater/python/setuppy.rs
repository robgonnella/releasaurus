use crate::{
    config::package::GENERIC_VERSION_REGEX,
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{generic::updater::GenericUpdater, traits::FileUpdater},
};

pub struct SetupPy {}

impl SetupPy {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SetupPy {
    fn default() -> Self {
        SetupPy::new()
    }
}

impl FileUpdater for SetupPy {
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename != "setup.py" {
            return Ok(None);
        }

        Ok(GenericUpdater::update_manifest(
            manifest,
            &GENERIC_VERSION_REGEX,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        config::release_type::ReleaseType,
        forge::request::Tag,
        packages::manifests::{ManifestFile, ManifestPackage},
    };

    use super::*;

    #[test]
    fn updates_version_with_double_quotes() {
        let setuppy = SetupPy::new();
        let content =
            "setup(\n    name='my-package',\n    version=\"1.0.0\",\n)\n";

        let manifest = ManifestFile {
            path: Path::new("setup.py").to_path_buf(),
            basename: "setup.py".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Python,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Python,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = setuppy.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version=\"2.0.0\""));
    }

    #[test]
    fn updates_version_with_single_quotes() {
        let setuppy = SetupPy::new();
        let content =
            "setup(\n    name='my-package',\n    version='1.0.0',\n)\n";

        let manifest = ManifestFile {
            path: Path::new("setup.py").to_path_buf(),
            basename: "setup.py".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Python,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Python,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = setuppy.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version='2.0.0'"));
    }

    #[test]
    fn preserves_whitespace_formatting() {
        let setuppy = SetupPy::new();
        let content =
            "setup(\n    name='my-package',\n    version   =   \"1.0.0\",\n)\n";

        let manifest = ManifestFile {
            path: Path::new("setup.py").to_path_buf(),
            basename: "setup.py".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Python,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Python,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = setuppy.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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
            path: Path::new("setup.py").to_path_buf(),
            basename: "setup.py".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Python,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Python,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = setuppy.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version=\"2.0.0\""));
        assert!(updated.contains("name='my-package'"));
        assert!(updated.contains("description='A test package'"));
        assert!(updated.contains("author='Test Author'"));
        assert!(updated.contains("packages=find_packages()"));
        assert!(updated.contains("'requests>=2.28.0'"));
    }

    #[test]
    fn process_package_returns_none_when_no_setup_py_files() {
        let setuppy = SetupPy::new();

        let manifest = ManifestFile {
            path: Path::new("setup.cfg").to_path_buf(),
            basename: "setup.cfg".to_string(),
            content: "[metadata]\nversion = 1.0.0\n".to_string(),
            release_type: ReleaseType::Python,
            owner: Some(ManifestPackage {
                name: "my-package".to_string(),
                release_type: ReleaseType::Python,
                tag: Tag {
                    name: "v2.0.0".into(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = setuppy.update(&manifest).unwrap();

        assert!(result.is_none());
    }
}
