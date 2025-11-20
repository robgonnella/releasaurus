// use log::*;
use regex::Regex;

use crate::{
    forge::request::{FileChange, FileUpdateType},
    result::Result,
    updater::framework::{ManifestFile, UpdaterPackage},
};

/// Handles version.rb file parsing and version updates for Ruby packages.
pub struct VersionRb {}

impl VersionRb {
    /// Create VersionRb handler for version.rb version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Process version.rb files for all Ruby packages.
    pub async fn process_package(
        &self,
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for manifest in package.manifest_files.iter() {
            if manifest.file_basename != "version.rb" {
                continue;
            }

            if let Some(change) = self.update_version(manifest, package) {
                file_changes.push(change);
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    /// Update version in a version.rb file.
    fn update_version(
        &self,
        manifest: &ManifestFile,
        package: &UpdaterPackage,
    ) -> Option<FileChange> {
        // Match patterns like:
        // VERSION = "1.0.0"
        // VERSION = '1.0.0'
        let re = Regex::new(r#"(VERSION\s*=\s*)(["'])([^"']+)(["'])"#).unwrap();

        if !re.is_match(&manifest.content) {
            return None;
        }

        let updated_content = re
            .replace_all(&manifest.content, |caps: &regex::Captures| {
                format!(
                    "{}{}{}{}",
                    &caps[1], &caps[2], package.next_version, &caps[4]
                )
            })
            .to_string();

        Some(FileChange {
            path: manifest.file_path.clone(),
            content: updated_content,
            update_type: FileUpdateType::Replace,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_helpers::create_test_tag,
        updater::framework::{Framework, ManifestFile, UpdaterPackage},
    };

    #[tokio::test]
    async fn updates_version_with_double_quotes() {
        let version_rb = VersionRb::new();
        let content = r#"module MyGem
  VERSION = "1.0.0"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "lib/my_gem/version.rb".to_string(),
            file_basename: "version.rb".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = version_rb.process_package(&package).await.unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("VERSION = \"v2.0.0\""));
    }

    #[tokio::test]
    async fn updates_version_with_single_quotes() {
        let version_rb = VersionRb::new();
        let content = r#"module MyGem
  VERSION = '1.0.0'
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "lib/my_gem/version.rb".to_string(),
            file_basename: "version.rb".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = version_rb.process_package(&package).await.unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("VERSION = 'v2.0.0'"));
    }

    #[tokio::test]
    async fn preserves_whitespace_formatting() {
        let version_rb = VersionRb::new();
        let content = r#"module MyGem
  VERSION   =   "1.0.0"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "lib/my_gem/version.rb".to_string(),
            file_basename: "version.rb".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = version_rb.process_package(&package).await.unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("VERSION   =   \"v2.0.0\""));
    }

    #[tokio::test]
    async fn returns_none_when_no_version_constant() {
        let version_rb = VersionRb::new();
        let content = r#"module MyGem
  AUTHOR = "Test Author"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "lib/my_gem/version.rb".to_string(),
            file_basename: "version.rb".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = version_rb.process_package(&package).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn preserves_other_content() {
        let version_rb = VersionRb::new();
        let content = r#"# frozen_string_literal: true

module MyGem
  # The current version
  VERSION = "1.0.0"

  # Other constants
  AUTHOR = "Test Author"
  HOMEPAGE = "https://example.com"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "lib/my_gem/version.rb".to_string(),
            file_basename: "version.rb".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = version_rb.process_package(&package).await.unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("VERSION = \"v2.0.0\""));
        assert!(updated.contains("# frozen_string_literal: true"));
        assert!(updated.contains("# The current version"));
        assert!(updated.contains("AUTHOR = \"Test Author\""));
        assert!(updated.contains("HOMEPAGE = \"https://example.com\""));
    }

    #[tokio::test]
    async fn process_package_handles_multiple_version_rb_files() {
        let version_rb = VersionRb::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            file_path: "gems/a/lib/gem_a/version.rb".to_string(),
            file_basename: "version.rb".to_string(),
            content: "module GemA\n  VERSION = \"1.0.0\"\nend\n".to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            file_path: "gems/b/lib/gem_b/version.rb".to_string(),
            file_basename: "version.rb".to_string(),
            content: "module GemB\n  VERSION = \"1.0.0\"\nend\n".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = version_rb.process_package(&package).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("v2.0.0")));
    }

    #[tokio::test]
    async fn process_package_returns_none_when_no_version_rb_files() {
        let version_rb = VersionRb::new();
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "lib/my_gem.rb".to_string(),
            file_basename: "my_gem.rb".to_string(),
            content: "module MyGem\n  # Main module\nend\n".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = version_rb.process_package(&package).await.unwrap();

        assert!(result.is_none());
    }
}
