use log::*;
use regex::Regex;

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
    updater::framework::UpdaterPackage,
};

/// Handles version.rb file parsing and version updates for Ruby packages.
pub struct VersionRb {}

impl VersionRb {
    /// Create VersionRb handler for version.rb version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version in a version.rb file.
    fn update_version(&self, content: &str, new_version: &str) -> String {
        // Match patterns like:
        // VERSION = "1.0.0"
        // VERSION = '1.0.0'
        let re = Regex::new(r#"(VERSION\s*=\s*)(["'])([^"']+)(["'])"#).unwrap();

        re.replace_all(content, |caps: &regex::Captures| {
            format!("{}{}{}{}", &caps[1], &caps[2], new_version, &caps[4])
        })
        .to_string()
    }

    /// Process version.rb files for all Ruby packages.
    pub async fn process_packages(
        &self,
        packages: &[UpdaterPackage],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for package in packages {
            if let Some(change) =
                self.process_version_file(package, loader).await?
            {
                file_changes.push(change);
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    /// Process version.rb file for a package.
    async fn process_version_file(
        &self,
        package: &UpdaterPackage,
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        // Try common version file locations
        let version_paths = vec![
            package.get_file_path(&format!("lib/{}/version.rb", package.name)),
            package.get_file_path("lib/version.rb"),
            package.get_file_path("version.rb"),
        ];

        for version_path in version_paths {
            let content = loader.get_file_content(&version_path).await?;

            if let Some(content) = content {
                info!("found version.rb file for package: {}", version_path);

                let updated_content = self.update_version(
                    &content,
                    &package.next_version.semver.to_string(),
                );

                if updated_content != content {
                    info!(
                        "updating {} version to {}",
                        version_path, package.next_version.semver
                    );

                    return Ok(Some(FileChange {
                        path: version_path,
                        content: updated_content,
                        update_type: FileUpdateType::Replace,
                    }));
                } else {
                    warn!(
                        "no version found to update in version file: {}",
                        version_path
                    );
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::traits::MockFileLoader;
    use crate::test_helpers::create_test_updater_package;
    use crate::updater::framework::Framework;

    #[test]
    fn test_update_version_double_quotes() {
        let version_rb = VersionRb::new();
        let version_rb_content = r#"
module MyGem
  VERSION = "1.0.0"
end
"#;

        let updated = version_rb.update_version(version_rb_content, "2.0.0");
        assert!(updated.contains(r#"VERSION = "2.0.0""#));
        assert!(!updated.contains(r#"VERSION = "1.0.0""#));
    }

    #[test]
    fn test_update_version_single_quotes() {
        let version_rb = VersionRb::new();
        let version_rb_content = r#"
module MyGem
  VERSION = '1.5.0'
end
"#;

        let updated = version_rb.update_version(version_rb_content, "2.0.0");
        assert!(updated.contains(r#"VERSION = '2.0.0'"#));
    }

    #[test]
    fn test_update_version_with_frozen_string() {
        let version_rb = VersionRb::new();
        let version_rb_content = r#"# frozen_string_literal: true

module MyGem
  VERSION = "1.0.0".freeze
end
"#;

        let updated = version_rb.update_version(version_rb_content, "3.0.0");
        assert!(updated.contains(r#"VERSION = "3.0.0""#));
    }

    #[test]
    fn test_preserves_spacing_around_equals() {
        let version_rb = VersionRb::new();

        // Test with spaces around =
        let version_with_spaces = r#"  VERSION = "1.0.0""#;
        let updated = version_rb.update_version(version_with_spaces, "2.0.0");
        assert!(updated.contains(r#"VERSION = "2.0.0""#));

        // Test with no spaces around =
        let version_no_spaces = r#"  VERSION="1.0.0""#;
        let updated = version_rb.update_version(version_no_spaces, "2.0.0");
        assert!(updated.contains(r#"VERSION="2.0.0""#));

        // Test with multiple spaces
        let version_multi_spaces = r#"  VERSION   =   "1.0.0""#;
        let updated = version_rb.update_version(version_multi_spaces, "2.0.0");
        assert!(updated.contains(r#"VERSION   =   "2.0.0""#));
    }

    #[test]
    fn test_preserves_indentation() {
        let version_rb = VersionRb::new();

        // Test with 2-space indentation
        let version_2_spaces = r#"
module MyGem
  VERSION = "1.0.0"
end
"#;
        let updated = version_rb.update_version(version_2_spaces, "2.0.0");
        assert!(updated.contains(r#"  VERSION = "2.0.0""#));

        // Test with 4-space indentation
        let version_4_spaces = r#"
module MyGem
    VERSION = "1.0.0"
end
"#;
        let updated = version_rb.update_version(version_4_spaces, "2.0.0");
        assert!(updated.contains(r#"    VERSION = "2.0.0""#));
    }

    #[tokio::test]
    async fn test_process_version_file_in_lib_gem_name() {
        let version_rb = VersionRb::new();
        let package = create_test_updater_package(
            "my-gem",
            "packages/my-gem",
            "2.0.0",
            Framework::Ruby,
        );

        let version_content = r#"
module MyGem
  VERSION = "1.0.0"
end
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "packages/my-gem/lib/my-gem/version.rb",
            ))
            .times(1)
            .returning({
                let content = version_content.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = version_rb
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/my-gem/lib/my-gem/version.rb");
        assert!(changes[0].content.contains(r#"VERSION = "2.0.0""#));
    }

    #[tokio::test]
    async fn test_process_version_file_fallback_to_lib_version() {
        let version_rb = VersionRb::new();
        let package = create_test_updater_package(
            "my-gem",
            "packages/my-gem",
            "3.0.0",
            Framework::Ruby,
        );

        let version_content = r#"
module MyGem
  VERSION = "1.0.0"
end
"#;

        let mut mock_loader = MockFileLoader::new();
        // First path not found
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "packages/my-gem/lib/my-gem/version.rb",
            ))
            .times(1)
            .returning(|_| Ok(None));

        // Second path found
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/my-gem/lib/version.rb"))
            .times(1)
            .returning({
                let content = version_content.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = version_rb
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/my-gem/lib/version.rb");
        assert!(changes[0].content.contains(r#"VERSION = "3.0.0""#));
    }

    #[tokio::test]
    async fn test_process_version_file_not_found() {
        let version_rb = VersionRb::new();
        let package = create_test_updater_package(
            "my-gem",
            "packages/my-gem",
            "2.0.0",
            Framework::Ruby,
        );

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = version_rb
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }
}
