use regex::Regex;
use std::sync::LazyLock;

use crate::{
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{generic::updater::GenericUpdater, traits::FileUpdater},
};

/// A `version` keyword argument or variable assigned a string literal,
/// anywhere on a line since `setup(name=…, version='…')` is often written
/// on one. Whatever the literal holds is replaced; `version=get_version()`
/// has nothing to match, and `\b` keeps `python_version` out.
static SETUP_PY_VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\bversion\s*=\s*['"](?<version>[^'"]*)['"]"#).unwrap()
});

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
            &SETUP_PY_VERSION_REGEX,
        ))
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use std::path::Path;

    use crate::{
        config::release_type::ReleaseType,
        forge::request::{Tag, TagVersion},
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
                    version: TagVersion::Semantic(
                        semver::Version::parse("2.0.0").unwrap(),
                    ),
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
                    version: TagVersion::Semantic(
                        semver::Version::parse("2.0.0").unwrap(),
                    ),
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
                    version: TagVersion::Semantic(
                        semver::Version::parse("2.0.0").unwrap(),
                    ),
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
                    version: TagVersion::Semantic(
                        semver::Version::parse("2.0.0").unwrap(),
                    ),
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
                    version: TagVersion::Semantic(
                        semver::Version::parse("2.0.0").unwrap(),
                    ),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let result = setuppy.update(&manifest).unwrap();

        assert!(result.is_none());
    }

    /// Whatever value is set is replaced, whatever its shape.
    #[test]
    fn updates_a_calver_version() {
        let updater = SetupPy::new();
        let manifest = ManifestFile {
            path: Path::new("setup.py").to_path_buf(),
            basename: "setup.py".to_string(),
            content: "setup(\n    version=\"26.09\",\n)\n".to_string(),
            release_type: ReleaseType::Python,
            owner: Some(ManifestPackage {
                name: "pkg".to_string(),
                release_type: ReleaseType::Python,
                tag: Tag {
                    name: "v26.10".into(),
                    version: TagVersion::Strftime(
                        Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap(),
                        "%y.%m".into(),
                    ),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        let updated = updater.update(&manifest).unwrap().unwrap().content;

        assert_eq!(updated, "setup(\n    version=\"26.10\",\n)\n");
    }

    fn update(content: &str) -> Option<String> {
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
                    version: TagVersion::Semantic(
                        semver::Version::parse("2.0.0").unwrap(),
                    ),
                    sha: "abc".into(),
                    ..Tag::default()
                },
            }),
            releasing: vec![],
        };

        SetupPy::new()
            .update(&manifest)
            .unwrap()
            .map(|change| change.content)
    }

    #[test]
    fn updates_a_one_line_setup_call() {
        let updated = update("setup(name='my-package', version='1.0.0')\n");

        assert_eq!(
            updated.as_deref(),
            Some("setup(name='my-package', version='2.0.0')\n")
        );
    }

    /// Only a string literal is ours to write: a call or reference is code,
    /// and `python_version` is a different keyword.
    #[test]
    fn leaves_non_literal_and_other_version_keywords_alone() {
        for content in [
            "setup(version=get_version())\n",
            "setup(version=__version__)\n",
            "setup(python_version='3.11.0')\n",
        ] {
            assert_eq!(update(content), None, "{content}");
        }
    }
}
