use regex::{Captures, Regex};
use std::sync::LazyLock;

use crate::{
    forge::request::{FileChange, FileUpdateType},
    packages::manifests::ManifestFile,
    result::Result,
    updater::{generic::updater::GenericUpdater, traits::FileUpdater},
};

/// A `version` option, quoted or not. Values are unquoted text, so the
/// regex alone cannot tell a version from a setuptools directive or a
/// `version` option in another section; `update` checks both.
static SETUP_CFG_VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^version\s*=\s*['"]?(?<version>[^'"\s#;]+)"#).unwrap()
});

static SECTION_HEADER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*\[(?<name>[^\]]+)\]").unwrap());

/// setuptools directives that read the version from elsewhere; replacing
/// one with a literal would discard where the version actually lives.
const DIRECTIVES: [&str; 2] = ["attr:", "file:"];

pub struct SetupCfg {}

impl SetupCfg {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SetupCfg {
    fn default() -> Self {
        SetupCfg::new()
    }
}

impl FileUpdater for SetupCfg {
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename != "setup.cfg" {
            return Ok(None);
        }

        let Some(owner) = manifest.owner.as_ref() else {
            return Ok(None);
        };

        let next_version = owner.tag.version.to_string();

        let content = SETUP_CFG_VERSION_REGEX
            .replace_all(&manifest.content, |caps: &Captures| {
                let full = caps.get(0).unwrap();
                let value = &caps["version"];

                let in_metadata = SECTION_HEADER_REGEX
                    .captures_iter(&manifest.content[..full.start()])
                    .last()
                    .is_some_and(|section| {
                        section["name"].trim() == "metadata"
                    });

                let is_directive =
                    DIRECTIVES.iter().any(|d| value.starts_with(d));

                if !in_metadata || is_directive {
                    return full.as_str().to_string();
                }

                GenericUpdater::replace_capture(caps, &next_version)
            })
            .to_string();

        if content == manifest.content {
            return Ok(None);
        }

        Ok(Some(FileChange {
            path: manifest.path.to_string_lossy().to_string(),
            content,
            update_type: FileUpdateType::Replace,
        }))
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
    fn updates_version_without_quotes() {
        let setupcfg = SetupCfg::new();
        let content = "[metadata]\nname = my-package\nversion = 1.0.0\n";

        let manifest = ManifestFile {
            path: Path::new("setup.cfg").to_path_buf(),
            basename: "setup.cfg".to_string(),
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

        let result = setupcfg.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version = 2.0.0"));
    }

    #[test]
    fn updates_version_with_double_quotes() {
        let setupcfg = SetupCfg::new();
        let content = "[metadata]\nname = my-package\nversion = \"1.0.0\"\n";

        let manifest = ManifestFile {
            path: Path::new("setup.cfg").to_path_buf(),
            basename: "setup.cfg".to_string(),
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

        let result = setupcfg.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version = \"2.0.0\""));
    }

    #[test]
    fn updates_version_with_single_quotes() {
        let setupcfg = SetupCfg::new();
        let content = "[metadata]\nname = my-package\nversion = '1.0.0'\n";

        let manifest = ManifestFile {
            path: Path::new("setup.cfg").to_path_buf(),
            basename: "setup.cfg".to_string(),
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

        let result = setupcfg.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version = '2.0.0'"));
    }

    #[test]
    fn preserves_whitespace_formatting() {
        let setupcfg = SetupCfg::new();
        let content = "[metadata]\nname = my-package\nversion   =   1.0.0\n";

        let manifest = ManifestFile {
            path: Path::new("setup.cfg").to_path_buf(),
            basename: "setup.cfg".to_string(),
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

        let result = setupcfg.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version   =   2.0.0"));
    }

    #[test]
    fn preserves_other_fields() {
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
            path: Path::new("setup.cfg").to_path_buf(),
            basename: "setup.cfg".to_string(),
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

        let result = setupcfg.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version = 2.0.0"));
        assert!(updated.contains("name = my-package"));
        assert!(updated.contains("description = A test package"));
        assert!(updated.contains("author = Test Author"));
        assert!(updated.contains("packages = find:"));
        assert!(updated.contains("requests>=2.28.0"));
    }

    #[test]
    fn process_package_returns_none_when_no_setup_cfg_files() {
        let setupcfg = SetupCfg::new();

        let manifest = ManifestFile {
            path: Path::new("setup.py").to_path_buf(),
            basename: "setup.py".to_string(),
            content: "setup(name='my-package', version='1.0.0')".to_string(),
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

        let result = setupcfg.update(&manifest).unwrap();

        assert!(result.is_none());
    }

    /// Whatever value is set is replaced, whatever its shape.
    #[test]
    fn updates_a_calver_version() {
        let updater = SetupCfg::new();
        let manifest = ManifestFile {
            path: Path::new("setup.cfg").to_path_buf(),
            basename: "setup.cfg".to_string(),
            content: "[metadata]\nversion = 26.09\n".to_string(),
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

        assert_eq!(updated, "[metadata]\nversion = 26.10\n");
    }

    fn update(content: &str) -> Option<String> {
        let manifest = ManifestFile {
            path: Path::new("setup.cfg").to_path_buf(),
            basename: "setup.cfg".to_string(),
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

        SetupCfg::new()
            .update(&manifest)
            .unwrap()
            .map(|change| change.content)
    }

    /// setuptools directives read the version from elsewhere.
    #[test]
    fn leaves_attr_and_file_directives_alone() {
        for value in ["attr: my_package.__version__", "file: VERSION"] {
            let content = format!("[metadata]\nversion = {value}\n");

            assert_eq!(update(&content), None, "{value}");
        }
    }

    #[test]
    fn only_updates_version_under_metadata() {
        let content = "[metadata]\nname = my-package\nversion = 1.0.0\n\n\
                       [tool:custom]\nversion = 1.0.0\n";

        assert_eq!(
            update(content).as_deref(),
            Some(
                "[metadata]\nname = my-package\nversion = 2.0.0\n\n\
                 [tool:custom]\nversion = 1.0.0\n"
            )
        );
    }

    #[test]
    fn leaves_version_outside_any_section_alone() {
        assert_eq!(update("version = 1.0.0\n"), None);
    }
}
