use regex::Regex;

use crate::{
    forge::request::{FileChange, FileUpdateType},
    packages::manifests::ManifestFile,
    result::Result,
    updater::traits::FileUpdater,
};

/// Handles yarn.lock file parsing and version updates for Node.js packages.
pub struct YarnLock {}

impl YarnLock {
    /// Create yarn.lock handler for version updates.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for YarnLock {
    fn default() -> Self {
        YarnLock::new()
    }
}

impl FileUpdater for YarnLock {
    /// Update version fields in yarn.lock files for all Node packages.
    fn update(&self, manifest: &ManifestFile) -> Result<Option<FileChange>> {
        // Regex to match package entries like "package@^1.0.0:"
        let package_regex = Regex::new(r#"^"?([^@"]+)@[^"]*"?:$"#)?;
        let version_regex = Regex::new(r#"^(\s+version\s+)"(.*)""#)?;

        if manifest.basename != "yarn.lock" {
            return Ok(None);
        }

        log::info!("processing {}", manifest.path.to_string_lossy());

        let mut updated = false;
        let mut lines: Vec<String> = vec![];

        let mut current_yarn_package: Option<String> = None;

        for line in manifest.content.lines() {
            // Check if this line starts a new package entry
            if let Some(caps) = package_regex.captures(line) {
                current_yarn_package = Some(caps[1].to_string());
                lines.push(line.to_string());
                continue;
            }

            // Check if this is a version line and we're in a relevant package
            if let (Some(pkg_name), Some(caps)) =
                (current_yarn_package.as_ref(), version_regex.captures(line))
                && let Some(pkg) =
                    manifest.releasing.iter().find(|p| p.name == *pkg_name)
            {
                let new_line = format!("{}\"{}\"", &caps[1], pkg.tag.semver);
                lines.push(new_line);
                updated = true;
                continue;
            }

            // Reset current package when we hit an empty line or start of new entry
            if line.trim().is_empty()
                || (!line.starts_with(' ')
                    && !line.starts_with('\t')
                    && line.contains(':'))
            {
                current_yarn_package = None;
            }

            lines.push(line.to_string());
        }

        let updated_content = lines.join("\n");

        if updated {
            return Ok(Some(FileChange {
                path: manifest.path.to_string_lossy().to_string(),
                content: updated_content,
                update_type: FileUpdateType::Replace,
            }));
        }

        Ok(None)
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

    fn package(name: &str, version: &str) -> ManifestPackage {
        ManifestPackage {
            name: name.to_string(),
            release_type: ReleaseType::Node,
            tag: Tag {
                name: format!("v{version}"),
                semver: semver::Version::parse(version).unwrap(),
                sha: "abc".into(),
                ..Tag::default()
            },
        }
    }

    /// `releasing` is owner-inclusive in production, so fixtures build it
    /// that way rather than naming the owner twice. `yarn.lock` reads
    /// nothing but `releasing`, which is why its own entry used to be
    /// skipped: the owner was filtered out of the peer list.
    fn manifest(
        path: &str,
        content: &str,
        owner: ManifestPackage,
        peers: Vec<ManifestPackage>,
    ) -> ManifestFile {
        let mut releasing = vec![owner.clone()];
        releasing.extend(peers);

        let path = Path::new(path);

        ManifestFile {
            basename: path.file_name().unwrap().to_string_lossy().to_string(),
            path: path.to_path_buf(),
            content: content.to_string(),
            release_type: ReleaseType::Node,
            owner: Some(owner),
            releasing,
        }
    }

    #[test]
    fn updates_workspace_package_version() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

"package-a@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.0.0.tgz"
"#;

        let manifest = manifest(
            "yarn.lock",
            content,
            package("package-a", "2.0.0"),
            vec![],
        );

        let result = yarn_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version \"2.0.0\""));
    }

    #[test]
    fn updates_multiple_workspace_packages() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

"package-a@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.0.0.tgz"

"package-b@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-b/-/package-b-1.0.0.tgz"
"#;

        let manifest = manifest(
            "yarn.lock",
            content,
            package("package-a", "2.0.0"),
            vec![package("package-b", "3.0.0")],
        );

        let result = yarn_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version \"2.0.0\""));
        assert!(updated.contains("version \"3.0.0\""));
    }

    /// A shared root `yarn.lock` has no owner when the workspace root is
    /// not being released. Members still get bumped.
    #[test]
    fn bumps_every_member_of_an_unowned_lock() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

"package-a@^1.0.0":
  version "1.0.0"

"package-b@^1.0.0":
  version "1.0.0"
"#;

        let manifest = ManifestFile {
            path: Path::new("yarn.lock").to_path_buf(),
            basename: "yarn.lock".to_string(),
            content: content.to_string(),
            release_type: ReleaseType::Node,
            owner: None,
            releasing: vec![
                package("package-a", "2.0.0"),
                package("package-b", "3.0.0"),
            ],
        };

        let result = yarn_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version \"2.0.0\""));
        assert!(updated.contains("version \"3.0.0\""));
    }

    #[test]
    fn preserves_non_workspace_packages() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

"package-a@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.0.0.tgz"

"external-lib@^5.0.0":
  version "5.0.0"
  resolved "https://registry.yarnpkg.com/external-lib/-/external-lib-5.0.0.tgz"
"#;

        let manifest = manifest(
            "yarn.lock",
            content,
            package("package-a", "2.0.0"),
            vec![package("package-b", "3.0.0")],
        );

        let result = yarn_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version \"2.0.0\""));
        assert!(updated.contains("version \"5.0.0\""));
    }

    #[test]
    fn handles_package_entries_without_quotes() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

package-a@^1.0.0:
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.0.0.tgz"
"#;

        let manifest = manifest(
            "yarn.lock",
            content,
            package("package-a", "2.0.0"),
            vec![],
        );

        let result = yarn_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("version \"2.0.0\""));
    }

    #[test]
    fn preserves_whitespace_formatting() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

"package-a@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.0.0.tgz"
  integrity sha512-abc123
"#;

        let manifest = manifest(
            "yarn.lock",
            content,
            package("package-a", "2.0.0"),
            vec![],
        );

        let result = yarn_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        assert!(updated.contains("  version \"2.0.0\""));
        assert!(updated.contains("  resolved"));
        assert!(updated.contains("  integrity"));
    }

    #[test]
    fn process_package_returns_none_when_no_yarn_lock_files() {
        let yarn_lock = YarnLock::new();

        let manifest = manifest(
            "package.json",
            r#"{"name":"my-package","version":"1.0.0"}"#,
            package("test", "2.0.0"),
            vec![],
        );

        assert!(yarn_lock.update(&manifest).unwrap().is_none());
    }

    #[test]
    fn returns_none_when_no_workspace_packages_to_update() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

"external-lib@^5.0.0":
  version "5.0.0"
  resolved "https://registry.yarnpkg.com/external-lib/-/external-lib-5.0.0.tgz"
"#;

        let manifest = manifest(
            "yarn.lock",
            content,
            package("package-a", "2.0.0"),
            vec![package("package-b", "3.0.0")],
        );

        assert!(yarn_lock.update(&manifest).unwrap().is_none());
    }

    #[test]
    fn handles_multiple_version_ranges_for_same_package() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

"package-a@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.0.0.tgz"

"package-a@^1.5.0":
  version "1.5.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.5.0.tgz"
"#;

        let manifest = manifest(
            "yarn.lock",
            content,
            package("package-a", "2.0.0"),
            vec![],
        );

        let result = yarn_lock.update(&manifest).unwrap();

        let updated = result.unwrap().content.clone();
        // Both entries should be updated to 2.0.0
        assert_eq!(updated.matches("version \"2.0.0\"").count(), 2);
    }
}
