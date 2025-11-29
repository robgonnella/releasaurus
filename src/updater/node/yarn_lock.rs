use log::*;
use regex::Regex;
// use std::collections::HashSet;

use crate::{
    cli::Result,
    forge::request::{FileChange, FileUpdateType},
    updater::framework::UpdaterPackage,
};

/// Handles yarn.lock file parsing and version updates for Node.js packages.
pub struct YarnLock {}

impl YarnLock {
    /// Create yarn.lock handler for version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version fields in yarn.lock files for all Node packages.
    pub fn process_package(
        &self,
        package: &UpdaterPackage,
        workspace_packages: &[UpdaterPackage],
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        // Regex to match package entries like "package@^1.0.0:"
        let package_regex = Regex::new(r#"^"?([^@"]+)@[^"]*"?:$"#)?;
        let version_regex = Regex::new(r#"^(\s+version\s+)"(.*)""#)?;

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename != "yarn.lock" {
                continue;
            }

            info!("processing {}", manifest.file_path);

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
                if let (Some(pkg_name), Some(caps)) = (
                    current_yarn_package.as_ref(),
                    version_regex.captures(line),
                ) && let Some(pkg) = workspace_packages
                    .iter()
                    .find(|p| p.package_name == *pkg_name)
                {
                    let new_line =
                        format!("{}\"{}\"", &caps[1], pkg.next_version.semver);
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
                file_changes.push(FileChange {
                    path: manifest.file_path.clone(),
                    content: updated_content,
                    update_type: FileUpdateType::Replace,
                });
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
    use std::slice;

    use super::*;
    use crate::{
        config::ManifestFile,
        test_helpers::create_test_tag,
        updater::framework::{Framework, UpdaterPackage},
    };

    #[tokio::test]
    async fn updates_workspace_package_version() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

"package-a@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.0.0.tgz"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "yarn.lock".to_string(),
            file_basename: "yarn.lock".to_string(),
            content: content.to_string(),
        };
        let package_a = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: "packages/a".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = yarn_lock
            .process_package(&package_a, slice::from_ref(&package_a))
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version \"2.0.0\""));
    }

    #[tokio::test]
    async fn updates_multiple_workspace_packages() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

"package-a@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.0.0.tgz"

"package-b@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-b/-/package-b-1.0.0.tgz"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "yarn.lock".to_string(),
            file_basename: "yarn.lock".to_string(),
            content: content.to_string(),
        };
        let package_a = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: "packages/a".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };
        let package_b = UpdaterPackage {
            package_name: "package-b".to_string(),
            workspace_root: "packages/b".to_string(),
            manifest_files: vec![],
            next_version: create_test_tag("v3.0.0", "3.0.0", "def"),
            framework: Framework::Node,
        };

        let result = yarn_lock
            .process_package(&package_a, &[package_a.clone(), package_b])
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version \"2.0.0\""));
        assert!(updated.contains("version \"3.0.0\""));
    }

    #[tokio::test]
    async fn preserves_non_workspace_packages() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

"package-a@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.0.0.tgz"

"external-lib@^5.0.0":
  version "5.0.0"
  resolved "https://registry.yarnpkg.com/external-lib/-/external-lib-5.0.0.tgz"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "yarn.lock".to_string(),
            file_basename: "yarn.lock".to_string(),
            content: content.to_string(),
        };
        let package_a = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: "packages/a".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = yarn_lock
            .process_package(&package_a, slice::from_ref(&package_a))
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version \"2.0.0\""));
        assert!(updated.contains("version \"5.0.0\""));
    }

    #[tokio::test]
    async fn handles_package_entries_without_quotes() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

package-a@^1.0.0:
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.0.0.tgz"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "yarn.lock".to_string(),
            file_basename: "yarn.lock".to_string(),
            content: content.to_string(),
        };
        let package_a = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: "packages/a".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = yarn_lock
            .process_package(&package_a, slice::from_ref(&package_a))
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("version \"2.0.0\""));
    }

    #[tokio::test]
    async fn preserves_whitespace_formatting() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

"package-a@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.0.0.tgz"
  integrity sha512-abc123
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "yarn.lock".to_string(),
            file_basename: "yarn.lock".to_string(),
            content: content.to_string(),
        };
        let package_a = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: "packages/a".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = yarn_lock
            .process_package(&package_a, slice::from_ref(&package_a))
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("  version \"2.0.0\""));
        assert!(updated.contains("  resolved"));
        assert!(updated.contains("  integrity"));
    }

    #[tokio::test]
    async fn process_package_handles_multiple_yarn_lock_files() {
        let yarn_lock = YarnLock::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            file_path: "packages/a/yarn.lock".to_string(),
            file_basename: "yarn.lock".to_string(),
            content: "\"package-a@^1.0.0\":\n  version \"1.0.0\"".to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            file_path: "packages/b/yarn.lock".to_string(),
            file_basename: "yarn.lock".to_string(),
            content: "\"package-a@^1.0.0\":\n  version \"1.0.0\"".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: "packages/a".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = yarn_lock
            .process_package(&package, slice::from_ref(&package))
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[tokio::test]
    async fn process_package_returns_none_when_no_yarn_lock_files() {
        let yarn_lock = YarnLock::new();
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "package.json".to_string(),
            file_basename: "package.json".to_string(),
            content: r#"{"name":"my-package","version":"1.0.0"}"#.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = yarn_lock.process_package(&package, &[]).unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn returns_none_when_no_workspace_packages_to_update() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

"external-lib@^5.0.0":
  version "5.0.0"
  resolved "https://registry.yarnpkg.com/external-lib/-/external-lib-5.0.0.tgz"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "yarn.lock".to_string(),
            file_basename: "yarn.lock".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-package".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = yarn_lock
            .process_package(&package, slice::from_ref(&package))
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn handles_multiple_version_ranges_for_same_package() {
        let yarn_lock = YarnLock::new();
        let content = r#"# yarn lockfile v1

"package-a@^1.0.0":
  version "1.0.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.0.0.tgz"

"package-a@^1.5.0":
  version "1.5.0"
  resolved "https://registry.yarnpkg.com/package-a/-/package-a-1.5.0.tgz"
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "yarn.lock".to_string(),
            file_basename: "yarn.lock".to_string(),
            content: content.to_string(),
        };
        let package_a = UpdaterPackage {
            package_name: "package-a".to_string(),
            workspace_root: "packages/a".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Node,
        };

        let result = yarn_lock
            .process_package(&package_a, slice::from_ref(&package_a))
            .unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        // Both entries should be updated to 2.0.0
        assert_eq!(updated.matches("version \"2.0.0\"").count(), 2);
    }
}
