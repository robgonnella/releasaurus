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

/// Handles .gemspec file parsing and version updates for Ruby packages.
pub struct Gemspec {}

impl Gemspec {
    /// Create Gemspec handler for .gemspec version updates.
    pub fn new() -> Self {
        Self {}
    }

    /// Update version string in gemspec file using regex pattern matching.
    fn update_version(&self, content: &str, new_version: &str) -> String {
        // Match patterns like:
        // spec.version = "1.0.0"
        // spec.version = '1.0.0'
        // s.version = "1.0.0"
        let re =
            Regex::new(r#"((?:spec|s)\.version\s*=\s*)(["'])([^"']+)(["'])"#)
                .unwrap();

        re.replace_all(content, |caps: &regex::Captures| {
            format!("{}{}{}{}", &caps[1], &caps[2], new_version, &caps[4])
        })
        .to_string()
    }

    /// Process gemspec files for all Ruby packages.
    pub async fn process_packages(
        &self,
        packages: &[UpdaterPackage],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for package in packages {
            if let Some(change) = self.process_gemspec(package, loader).await? {
                file_changes.push(change);
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    /// Process a single gemspec file for a package.
    async fn process_gemspec(
        &self,
        package: &UpdaterPackage,
        loader: &dyn FileLoader,
    ) -> Result<Option<FileChange>> {
        // Look for *.gemspec files
        let gemspec_path =
            package.get_file_path(&format!("{}.gemspec", package.name));

        let content = loader.get_file_content(&gemspec_path).await?;

        if let Some(content) = content {
            info!("found gemspec file for package: {}", gemspec_path);

            let updated_content = self.update_version(
                &content,
                &package.next_version.semver.to_string(),
            );

            if updated_content != content {
                info!(
                    "updating {} version to {}",
                    gemspec_path, package.next_version.semver
                );

                return Ok(Some(FileChange {
                    path: gemspec_path,
                    content: updated_content,
                    update_type: FileUpdateType::Replace,
                }));
            } else {
                warn!(
                    "no version found to update in gemspec: {}",
                    gemspec_path
                );
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
        let gemspec = Gemspec::new();
        let gemspec_content = r#"
Gem::Specification.new do |spec|
  spec.name = "my-gem"
  spec.version = "1.0.0"
  spec.authors = ["John Doe"]
end
"#;

        let updated = gemspec.update_version(gemspec_content, "2.0.0");
        assert!(updated.contains(r#"spec.version = "2.0.0""#));
        assert!(!updated.contains(r#"spec.version = "1.0.0""#));
    }

    #[test]
    fn test_update_version_single_quotes() {
        let gemspec = Gemspec::new();
        let gemspec_content = r#"
Gem::Specification.new do |spec|
  spec.name = 'my-gem'
  spec.version = '1.0.0'
  spec.authors = ['John Doe']
end
"#;

        let updated = gemspec.update_version(gemspec_content, "2.5.0");
        assert!(updated.contains(r#"spec.version = '2.5.0'"#));
    }

    #[test]
    fn test_update_version_short_form() {
        let gemspec = Gemspec::new();
        let gemspec_content = r#"
Gem::Specification.new do |s|
  s.name = "my-gem"
  s.version = "1.0.0"
  s.authors = ["John Doe"]
end
"#;

        let updated = gemspec.update_version(gemspec_content, "3.0.0");
        assert!(updated.contains(r#"s.version = "3.0.0""#));
    }

    #[test]
    fn test_preserves_spacing_around_equals() {
        let gemspec = Gemspec::new();

        // Test with spaces around =
        let gemspec_with_spaces = r#"
  spec.version = "1.0.0"
"#;
        let updated = gemspec.update_version(gemspec_with_spaces, "2.0.0");
        assert!(updated.contains(r#"spec.version = "2.0.0""#));

        // Test with no spaces around =
        let gemspec_no_spaces = r#"
  spec.version="1.0.0"
"#;
        let updated = gemspec.update_version(gemspec_no_spaces, "2.0.0");
        assert!(updated.contains(r#"spec.version="2.0.0""#));

        // Test with multiple spaces
        let gemspec_multi_spaces = r#"
  spec.version   =   "1.0.0"
"#;
        let updated = gemspec.update_version(gemspec_multi_spaces, "2.0.0");
        assert!(updated.contains(r#"spec.version   =   "2.0.0""#));
    }

    #[test]
    fn test_preserves_indentation() {
        let gemspec = Gemspec::new();

        // Test with 2-space indentation
        let gemspec_2_spaces = r#"
Gem::Specification.new do |spec|
  spec.version = "1.0.0"
end
"#;
        let updated = gemspec.update_version(gemspec_2_spaces, "2.0.0");
        assert!(updated.contains(r#"  spec.version = "2.0.0""#));

        // Test with 4-space indentation
        let gemspec_4_spaces = r#"
Gem::Specification.new do |spec|
    spec.version = "1.0.0"
end
"#;
        let updated = gemspec.update_version(gemspec_4_spaces, "2.0.0");
        assert!(updated.contains(r#"    spec.version = "2.0.0""#));

        // Test with tab indentation
        let gemspec_tabs = r#"
Gem::Specification.new do |spec|
	spec.version = "1.0.0"
end
"#;
        let updated = gemspec.update_version(gemspec_tabs, "2.0.0");
        assert!(updated.contains("	spec.version = \"2.0.0\""));
    }

    #[tokio::test]
    async fn test_process_gemspec_updates_version() {
        let gemspec = Gemspec::new();
        let package = create_test_updater_package(
            "my-gem",
            "packages/my-gem",
            "2.0.0",
            Framework::Ruby,
        );

        let gemspec_content = r#"
Gem::Specification.new do |spec|
  spec.name = "my-gem"
  spec.version = "1.0.0"
  spec.summary = "A test gem"
end
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/my-gem/my-gem.gemspec"))
            .times(1)
            .returning({
                let content = gemspec_content.to_string();
                move |_| Ok(Some(content.clone()))
            });

        let packages = vec![package];
        let result = gemspec
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/my-gem/my-gem.gemspec");
        assert!(changes[0].content.contains(r#"spec.version = "2.0.0""#));
    }

    #[tokio::test]
    async fn test_process_gemspec_not_found() {
        let gemspec = Gemspec::new();
        let package = create_test_updater_package(
            "my-gem",
            "packages/my-gem",
            "2.0.0",
            Framework::Ruby,
        );

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/my-gem/my-gem.gemspec"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = gemspec
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }
}
