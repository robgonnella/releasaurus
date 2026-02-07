use toml_edit::DocumentMut;

use crate::{
    Result,
    forge::request::{FileChange, FileUpdateType},
    updater::{manager::UpdaterPackage, traits::PackageUpdater},
};

/// Handles gradle/libs.versions.toml (Gradle Version Catalog) parsing and
/// version updates for Java packages. Looks for a key matching the package
/// name in the [versions] section and updates its value to the next version.
pub struct LibsVersionsToml {}

impl LibsVersionsToml {
    pub fn new() -> Self {
        Self {}
    }

    fn load_doc(&self, content: &str) -> Result<DocumentMut> {
        let doc = content.parse::<DocumentMut>()?;
        Ok(doc)
    }
}

impl PackageUpdater for LibsVersionsToml {
    fn update(
        &self,
        package: &UpdaterPackage,
        _workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.basename != "libs.versions.toml" {
                continue;
            }

            let mut doc = self.load_doc(&manifest.content)?;

            let Some(versions) =
                doc.get_mut("versions").and_then(|v| v.as_table_like_mut())
            else {
                continue;
            };

            let Some(version_key) =
                find_version_key(versions, &package.package_name)
            else {
                continue;
            };

            let next_version = package.next_version.semver.to_string();

            log::info!(
                "setting version for {} to {next_version} in libs.versions.toml (key: {version_key})",
                package.package_name
            );

            if let Some(item) = versions.get_mut(&version_key) {
                if item.is_str() {
                    // Replace the raw string value in the existing decorated
                    // value, preserving comments and formatting.
                    let decorated = item.as_value_mut().unwrap();
                    let mut new_val =
                        toml_edit::Value::from(next_version.as_str());
                    // Copy original decorations (prefix whitespace, suffix,
                    // comments) onto the new value.
                    *new_val.decor_mut() = decorated.decor().clone();
                    *decorated = new_val;
                } else {
                    log::debug!(
                        "skipping non-string version key '{}' in libs.versions.toml",
                        version_key
                    );
                    continue;
                }
            }

            file_changes.push(FileChange {
                path: manifest.path.to_string_lossy().to_string(),
                content: doc.to_string(),
                update_type: FileUpdateType::Replace,
            });
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}

/// Normalizes a string by removing hyphens and underscores, then lowercasing.
/// This allows matching package names like "my-app" against TOML keys like
/// "myApp", "my_app", or "my-app".
fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '-' && *c != '_')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Finds a key in the [versions] table that matches the package name after
/// normalization. Returns the original key string so it can be used for
/// insertion (preserving the user's chosen casing/style).
fn find_version_key(
    versions: &dyn toml_edit::TableLike,
    package_name: &str,
) -> Option<String> {
    let normalized_name = normalize(package_name);

    for (key, _) in versions.iter() {
        if normalize(key) == normalized_name {
            return Some(key.to_string());
        }
    }

    None
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

    fn make_package(
        name: &str,
        version: &str,
        manifests: Vec<ManifestFile>,
    ) -> UpdaterPackage {
        UpdaterPackage {
            package_name: name.to_string(),
            manifest_files: manifests,
            next_version: Tag {
                name: format!("v{version}"),
                semver: semver::Version::parse(version).unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
            updater: Rc::new(Updater::new(ReleaseType::Java)),
        }
    }

    fn make_manifest(content: &str) -> ManifestFile {
        ManifestFile {
            path: Path::new("gradle/libs.versions.toml").to_path_buf(),
            basename: "libs.versions.toml".to_string(),
            content: content.to_string(),
        }
    }

    #[test]
    fn updates_version_matching_package_name() {
        let updater = LibsVersionsToml::new();
        let content = r#"[versions]
my-app = "1.0.0"
kotlin = "1.9.20"

[libraries]
kotlin-stdlib = { module = "org.jetbrains.kotlin:kotlin-stdlib", version.ref = "kotlin" }
"#;
        let package =
            make_package("my-app", "2.0.0", vec![make_manifest(content)]);

        let result = updater.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].content.contains("my-app = \"2.0.0\""));
        assert!(
            changes[0].content.contains("kotlin = \"1.9.20\""),
            "other versions should not be updated"
        );
    }

    #[test]
    fn matches_camel_case_key() {
        let updater = LibsVersionsToml::new();
        let content = r#"[versions]
myApp = "1.0.0"
kotlin = "1.9.20"
"#;
        let package =
            make_package("my-app", "2.0.0", vec![make_manifest(content)]);

        let result = updater.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].content.contains("myApp = \"2.0.0\""));
    }

    #[test]
    fn matches_underscore_key() {
        let updater = LibsVersionsToml::new();
        let content = r#"[versions]
my_app = "1.0.0"
"#;
        let package =
            make_package("my-app", "3.0.0", vec![make_manifest(content)]);

        let result = updater.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].content.contains("my_app = \"3.0.0\""));
    }

    #[test]
    fn returns_none_when_no_matching_key() {
        let updater = LibsVersionsToml::new();
        let content = r#"[versions]
kotlin = "1.9.20"
spring-boot = "3.2.0"
"#;
        let package =
            make_package("my-app", "2.0.0", vec![make_manifest(content)]);

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn returns_none_when_no_versions_section() {
        let updater = LibsVersionsToml::new();
        let content = r#"[libraries]
kotlin-stdlib = { module = "org.jetbrains.kotlin:kotlin-stdlib" }
"#;
        let package =
            make_package("my-app", "2.0.0", vec![make_manifest(content)]);

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn returns_none_for_non_libs_versions_toml_files() {
        let updater = LibsVersionsToml::new();
        let manifest = ManifestFile {
            path: Path::new("build.gradle").to_path_buf(),
            basename: "build.gradle".to_string(),
            content: "version = \"1.0.0\"".to_string(),
        };
        let package = make_package("my-app", "2.0.0", vec![manifest]);

        let result = updater.update(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn preserves_formatting_and_comments() {
        let updater = LibsVersionsToml::new();
        let content = r#"# Version catalog for my project
[versions]
# Project version
my-app = "1.0.0"
# Kotlin version
kotlin = "1.9.20"

[libraries]
kotlin-stdlib = { module = "org.jetbrains.kotlin:kotlin-stdlib", version.ref = "kotlin" }
"#;
        let package =
            make_package("my-app", "2.0.0", vec![make_manifest(content)]);

        let result = updater.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        let updated = &changes[0].content;
        assert!(updated.contains("my-app = \"2.0.0\""));
        assert!(updated.contains("# Project version"));
        assert!(updated.contains("# Kotlin version"));
        assert!(updated.contains("kotlin = \"1.9.20\""));
        assert!(updated.contains("[libraries]"));
    }

    #[test]
    fn case_insensitive_matching() {
        let updater = LibsVersionsToml::new();
        let content = r#"[versions]
MyApp = "1.0.0"
"#;
        let package =
            make_package("my-app", "2.0.0", vec![make_manifest(content)]);

        let result = updater.update(&package, &[]).unwrap();

        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].content.contains("MyApp = \"2.0.0\""));
    }

    #[test]
    fn normalize_handles_various_formats() {
        assert_eq!(normalize("my-app"), "myapp");
        assert_eq!(normalize("my_app"), "myapp");
        assert_eq!(normalize("myApp"), "myapp");
        assert_eq!(normalize("MyApp"), "myapp");
        assert_eq!(normalize("my-cool-app"), "mycoolapp");
        assert_eq!(normalize("my_cool_app"), "mycoolapp");
        assert_eq!(normalize("myCoolApp"), "mycoolapp");
    }
}
