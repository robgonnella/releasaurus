use regex::Regex;
use std::sync::LazyLock;

use crate::{
    config::release_type::ReleaseType,
    forge::request::FileChange,
    packages::manifests::ManifestFile,
    result::Result,
    updater::{generic::updater::GenericUpdater, traits::FileUpdater},
};

/// A `Version` (any case) constant or variable assigned a string literal,
/// standalone (`const Version = "…"`) or inside a `const (…)` block.
/// Whatever the literal holds is replaced; a `Version` without a literal
/// (`var Version string`, set by `-ldflags`) has nothing to match.
static VERSION_GO_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?m)^\s*(?:(?:const|var)\s+)?(?i:version)\s*(?:string\s*)?=\s*"(?<version>[^"]*)""#,
    )
    .unwrap()
});

/// Handles version.go file parsing and version updates for Golang packages.
pub struct VersionGo {}

impl VersionGo {
    /// Create VersionGo handler for version.go version updates.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for VersionGo {
    fn default() -> Self {
        VersionGo::new()
    }
}

impl FileUpdater for VersionGo {
    /// Process version.go files for all Golang packages.
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        if manifest.basename != "version.go"
            || !matches!(manifest.release_type, ReleaseType::Go)
        {
            return Ok(None);
        }

        Ok(GenericUpdater::update_manifest(manifest, &VERSION_GO_REGEX))
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use std::path::Path;

    use semver::Version;

    use crate::{
        config::release_type::ReleaseType,
        forge::request::{Tag, TagVersion},
        packages::manifests::{ManifestFile, ManifestPackage},
    };

    use super::*;

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
            release_type: ReleaseType::Go,
            owner: Some(ManifestPackage {
                name: "gopher".to_string(),
                release_type: ReleaseType::Go,
                tag: Tag {
                    name: "v2.0.0".into(),
                    version: TagVersion::Semantic(Version::new(2, 0, 0)),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_go.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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
            release_type: ReleaseType::Go,
            owner: Some(ManifestPackage {
                name: "gopher".to_string(),
                release_type: ReleaseType::Go,
                tag: Tag {
                    name: "v2.0.0".into(),
                    version: TagVersion::Semantic(Version::new(2, 0, 0)),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_go.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
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
            release_type: ReleaseType::Go,
            owner: Some(ManifestPackage {
                name: "gopher".to_string(),
                release_type: ReleaseType::Go,
                tag: Tag {
                    name: "v2.0.0".into(),
                    version: TagVersion::Semantic(Version::new(2, 0, 0)),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_go.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("const VERSION = \"2.0.0\""));
    }

    #[test]
    fn updates_all_caps_version_2() {
        let version_go = VersionGo::new();

        let content = "package internal\n\nconst VERSION = \"0.0.1\"\n";
        let manifest = ManifestFile {
            path: Path::new("internal/version.go").to_path_buf(),
            basename: "version.go".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Go,
            owner: Some(ManifestPackage {
                name: "gopher".to_string(),
                release_type: ReleaseType::Go,
                tag: Tag {
                    name: "v2.0.0".into(),
                    version: TagVersion::Semantic(Version::new(2, 0, 0)),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        let result = version_go.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("const VERSION = \"2.0.0\""));
    }

    /// Whatever value is set is replaced, whatever its shape.
    #[test]
    fn updates_a_calver_version() {
        let updater = VersionGo::new();
        let manifest = ManifestFile {
            path: Path::new("version.go").to_path_buf(),
            basename: "version.go".to_string(),
            content: "const Version = \"26.09\"\n".to_string(),
            release_type: ReleaseType::Go,
            owner: Some(ManifestPackage {
                name: "pkg".to_string(),
                release_type: ReleaseType::Go,
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

        assert_eq!(updated, "const Version = \"26.10\"\n");
    }

    fn update(content: &str) -> Option<String> {
        let manifest = ManifestFile {
            path: Path::new("version.go").to_path_buf(),
            basename: "version.go".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Go,
            owner: Some(ManifestPackage {
                name: "gopher".to_string(),
                release_type: ReleaseType::Go,
                tag: Tag {
                    name: "v2.0.0".into(),
                    version: TagVersion::Semantic(Version::new(2, 0, 0)),
                    sha: "abc".into(),
                    ..Default::default()
                },
            }),
            releasing: vec![],
        };

        VersionGo::new()
            .update(&manifest)
            .unwrap()
            .map(|change| change.content)
    }

    #[test]
    fn updates_version_in_a_const_block() {
        let updated =
            update("const (\n\tName = \"app\"\n\tVersion = \"1.0.0\"\n)\n");

        assert_eq!(
            updated.as_deref(),
            Some("const (\n\tName = \"app\"\n\tVersion = \"2.0.0\"\n)\n")
        );
    }

    #[test]
    fn updates_a_typed_var() {
        let updated = update("var Version string = \"1.0.0\"\n");

        assert_eq!(
            updated.as_deref(),
            Some("var Version string = \"2.0.0\"\n")
        );
    }

    /// Without a string literal there is no value of ours to write: an
    /// ldflags-set variable, or one assigned from an expression.
    #[test]
    fn leaves_versions_without_a_literal_alone() {
        for content in [
            "var Version string\n",
            "var Version = buildVersion()\n",
            "const AppVersion = \"1.0.0\"\n",
        ] {
            assert_eq!(update(content), None, "{content}");
        }
    }
}
