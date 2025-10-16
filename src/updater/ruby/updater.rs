use async_trait::async_trait;
use log::*;
use regex::Regex;

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
    updater::{
        framework::{Framework, UpdaterPackage},
        traits::PackageUpdater,
    },
};

/// Ruby package updater for Gem and Bundler projects.
pub struct RubyUpdater {}

impl RubyUpdater {
    /// Create Ruby updater for Gem and Bundler projects.
    pub fn new() -> Self {
        Self {}
    }

    /// Load file content from repository by path.
    async fn load_file(
        &self,
        file_path: &str,
        loader: &dyn FileLoader,
    ) -> Result<Option<String>> {
        loader.get_file_content(file_path).await
    }

    /// Update version string in gemspec file using regex pattern matching.
    fn update_gemspec_version(
        &self,
        content: &str,
        new_version: &str,
    ) -> String {
        // Match patterns like:
        // spec.version = "1.0.0"
        // spec.version = '1.0.0'
        // s.version = "1.0.0"
        let re = Regex::new(r#"((?:spec|s)\.version\s*=\s*)["']([^"']+)["']"#)
            .unwrap();

        re.replace_all(content, |caps: &regex::Captures| {
            format!(r#"{}"{}""#, &caps[1], new_version)
        })
        .to_string()
    }

    /// Update version in a version.rb file
    fn update_version_file(&self, content: &str, new_version: &str) -> String {
        // Match patterns like:
        // VERSION = "1.0.0"
        // VERSION = '1.0.0'
        let re = Regex::new(r#"(VERSION\s*=\s*)["']([^"']+)["']"#).unwrap();

        re.replace_all(content, |caps: &regex::Captures| {
            format!(r#"{}"{}""#, &caps[1], new_version)
        })
        .to_string()
    }

    /// Process packages and update their Ruby version files
    async fn process_packages(
        &self,
        packages: &[UpdaterPackage],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for package in packages {
            // Try to update gemspec file
            if let Some(changes) = self.process_gemspec(package, loader).await?
            {
                file_changes.extend(changes);
            }

            // Try to update version.rb file
            if let Some(changes) =
                self.process_version_file(package, loader).await?
            {
                file_changes.extend(changes);
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    /// Process gemspec file for a package
    async fn process_gemspec(
        &self,
        package: &UpdaterPackage,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        // Look for *.gemspec files
        let gemspec_path =
            package.get_file_path(&format!("{}.gemspec", package.name));

        let content = self.load_file(&gemspec_path, loader).await?;

        if let Some(content) = content {
            info!("found gemspec file for package: {}", gemspec_path);

            let updated_content = self.update_gemspec_version(
                &content,
                &package.next_version.semver.to_string(),
            );

            if updated_content != content {
                info!(
                    "updating {} version to {}",
                    gemspec_path, package.next_version.semver
                );

                return Ok(Some(vec![FileChange {
                    path: gemspec_path,
                    content: updated_content,
                    update_type: FileUpdateType::Replace,
                }]));
            } else {
                warn!(
                    "no version found to update in gemspec: {}",
                    gemspec_path
                );
            }
        }

        Ok(None)
    }

    /// Process version.rb file for a package
    async fn process_version_file(
        &self,
        package: &UpdaterPackage,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        // Try common version file locations
        let version_paths = vec![
            package.get_file_path(&format!("lib/{}/version.rb", package.name)),
            package.get_file_path("lib/version.rb"),
            package.get_file_path("version.rb"),
        ];

        for version_path in version_paths {
            let content = self.load_file(&version_path, loader).await?;

            if let Some(content) = content {
                info!("found version.rb file for package: {}", version_path);

                let updated_content = self.update_version_file(
                    &content,
                    &package.next_version.semver.to_string(),
                );

                if updated_content != content {
                    info!(
                        "updating {} version to {}",
                        version_path, package.next_version.semver
                    );

                    return Ok(Some(vec![FileChange {
                        path: version_path,
                        content: updated_content,
                        update_type: FileUpdateType::Replace,
                    }]));
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

#[async_trait]
impl PackageUpdater for RubyUpdater {
    async fn update(
        &self,
        packages: Vec<UpdaterPackage>,
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let ruby_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Ruby))
            .collect::<Vec<UpdaterPackage>>();

        info!("Found {} Ruby packages", ruby_packages.len());

        if ruby_packages.is_empty() {
            return Ok(None);
        }

        self.process_packages(&ruby_packages, loader).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::release::Tag;
    use crate::forge::traits::MockFileLoader;
    use crate::test_helpers::create_test_updater_package;
    use semver::Version as SemVer;

    #[test]
    fn test_update_gemspec_version_double_quotes() {
        let updater = RubyUpdater::new();
        let gemspec = r#"
Gem::Specification.new do |spec|
  spec.name = "my-gem"
  spec.version = "1.0.0"
  spec.authors = ["John Doe"]
end
"#;

        let updated = updater.update_gemspec_version(gemspec, "2.0.0");
        assert!(updated.contains(r#"spec.version = "2.0.0""#));
        assert!(!updated.contains(r#"spec.version = "1.0.0""#));
    }

    #[test]
    fn test_update_gemspec_version_single_quotes() {
        let updater = RubyUpdater::new();
        let gemspec = r#"
Gem::Specification.new do |spec|
  spec.name = 'my-gem'
  spec.version = '1.0.0'
  spec.authors = ['John Doe']
end
"#;

        let updated = updater.update_gemspec_version(gemspec, "2.5.0");
        assert!(updated.contains(r#"spec.version = "2.5.0""#));
    }

    #[test]
    fn test_update_gemspec_version_short_form() {
        let updater = RubyUpdater::new();
        let gemspec = r#"
Gem::Specification.new do |s|
  s.name = "my-gem"
  s.version = "1.0.0"
  s.authors = ["John Doe"]
end
"#;

        let updated = updater.update_gemspec_version(gemspec, "3.0.0");
        assert!(updated.contains(r#"s.version = "3.0.0""#));
    }

    #[test]
    fn test_update_version_file_double_quotes() {
        let updater = RubyUpdater::new();
        let version_rb = r#"
module MyGem
  VERSION = "1.0.0"
end
"#;

        let updated = updater.update_version_file(version_rb, "2.0.0");
        assert!(updated.contains(r#"VERSION = "2.0.0""#));
        assert!(!updated.contains(r#"VERSION = "1.0.0""#));
    }

    #[test]
    fn test_update_version_file_single_quotes() {
        let updater = RubyUpdater::new();
        let version_rb = r#"
module MyGem
  VERSION = '1.5.0'
end
"#;

        let updated = updater.update_version_file(version_rb, "2.0.0");
        assert!(updated.contains(r#"VERSION = "2.0.0""#));
    }

    #[test]
    fn test_update_version_file_with_frozen_string() {
        let updater = RubyUpdater::new();
        let version_rb = r#"# frozen_string_literal: true

module MyGem
  VERSION = "1.0.0".freeze
end
"#;

        let updated = updater.update_version_file(version_rb, "3.0.0");
        assert!(updated.contains(r#"VERSION = "3.0.0""#));
    }

    #[tokio::test]
    async fn test_process_gemspec_updates_version() {
        let updater = RubyUpdater::new();
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
  spec.authors = ["John Doe"]
  spec.summary = "A test gem"
end
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/my-gem/my-gem.gemspec"))
            .times(1)
            .returning(move |_| Ok(Some(gemspec_content.to_string())));

        let result = updater
            .process_gemspec(&package, &mock_loader)
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
        let updater = RubyUpdater::new();
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

        let result = updater
            .process_gemspec(&package, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_process_version_file_in_lib_gem_name() {
        let updater = RubyUpdater::new();
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
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "packages/my-gem/lib/my-gem/version.rb",
            ))
            .times(1)
            .returning(move |_| Ok(Some(version_content.to_string())));

        let result = updater
            .process_version_file(&package, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/my-gem/lib/my-gem/version.rb");
        assert!(changes[0].content.contains(r#"VERSION = "3.0.0""#));
    }

    #[tokio::test]
    async fn test_process_version_file_fallback_to_lib_version() {
        let updater = RubyUpdater::new();
        let package = create_test_updater_package(
            "my-gem",
            "packages/my-gem",
            "2.5.0",
            Framework::Ruby,
        );

        let version_content = r#"
VERSION = "1.0.0"
"#;

        let mut mock_loader = MockFileLoader::new();
        // First location not found
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "packages/my-gem/lib/my-gem/version.rb",
            ))
            .times(1)
            .returning(|_| Ok(None));

        // Second location found
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/my-gem/lib/version.rb"))
            .times(1)
            .returning(move |_| Ok(Some(version_content.to_string())));

        let result = updater
            .process_version_file(&package, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/my-gem/lib/version.rb");
        assert!(changes[0].content.contains(r#"VERSION = "2.5.0""#));
    }

    #[tokio::test]
    async fn test_process_packages_updates_both_files() {
        let updater = RubyUpdater::new();
        let package = create_test_updater_package(
            "my-gem",
            "packages/my-gem",
            "2.0.0",
            Framework::Ruby,
        );

        let gemspec_content = r#"spec.version = "1.0.0""#;
        let version_content = r#"VERSION = "1.0.0""#;

        let mut mock_loader = MockFileLoader::new();

        // Gemspec
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/my-gem/my-gem.gemspec"))
            .times(1)
            .returning(move |_| Ok(Some(gemspec_content.to_string())));

        // Version file
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "packages/my-gem/lib/my-gem/version.rb",
            ))
            .times(1)
            .returning(move |_| Ok(Some(version_content.to_string())));

        let packages = vec![package];
        let result = updater
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);

        // Both files should be updated
        assert!(
            changes
                .iter()
                .any(|c| c.path == "packages/my-gem/my-gem.gemspec")
        );
        assert!(
            changes
                .iter()
                .any(|c| c.path == "packages/my-gem/lib/my-gem/version.rb")
        );
    }

    #[tokio::test]
    async fn test_process_packages_multiple_packages() {
        let updater = RubyUpdater::new();
        let packages = vec![
            create_test_updater_package(
                "gem-one",
                "packages/one",
                "2.0.0",
                Framework::Ruby,
            ),
            create_test_updater_package(
                "gem-two",
                "packages/two",
                "3.0.0",
                Framework::Ruby,
            ),
        ];

        let gemspec1 = r#"spec.version = "1.0.0""#;
        let gemspec2 = r#"spec.version = "1.0.0""#;

        let mut mock_loader = MockFileLoader::new();

        // First package
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/one/gem-one.gemspec"))
            .times(1)
            .returning(move |_| Ok(Some(gemspec1.to_string())));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "packages/one/lib/gem-one/version.rb",
            ))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/one/lib/version.rb"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/one/version.rb"))
            .times(1)
            .returning(|_| Ok(None));

        // Second package
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/two/gem-two.gemspec"))
            .times(1)
            .returning(move |_| Ok(Some(gemspec2.to_string())));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "packages/two/lib/gem-two/version.rb",
            ))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/two/lib/version.rb"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/two/version.rb"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);

        let change1 = changes
            .iter()
            .find(|c| c.path == "packages/one/gem-one.gemspec")
            .unwrap();
        assert!(change1.content.contains(r#"spec.version = "2.0.0""#));

        let change2 = changes
            .iter()
            .find(|c| c.path == "packages/two/gem-two.gemspec")
            .unwrap();
        assert!(change2.content.contains(r#"spec.version = "3.0.0""#));
    }

    #[tokio::test]
    async fn test_process_packages_no_files_found() {
        let updater = RubyUpdater::new();
        let package = create_test_updater_package(
            "my-gem",
            "packages/my-gem",
            "2.0.0",
            Framework::Ruby,
        );

        let mut mock_loader = MockFileLoader::new();

        // Gemspec not found
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/my-gem/my-gem.gemspec"))
            .times(1)
            .returning(|_| Ok(None));

        // Version files not found
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "packages/my-gem/lib/my-gem/version.rb",
            ))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/my-gem/lib/version.rb"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/my-gem/version.rb"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_filters_ruby_packages() {
        let updater = RubyUpdater::new();

        let packages = vec![
            create_test_updater_package(
                "ruby-gem",
                "packages/ruby",
                "2.0.0",
                Framework::Ruby,
            ),
            UpdaterPackage {
                name: "node-package".into(),
                path: "packages/node".into(),
                workspace_root: ".".into(),
                framework: Framework::Node,
                next_version: Tag {
                    sha: "test-sha".into(),
                    name: "v1.0.0".into(),
                    semver: SemVer::parse("1.0.0").unwrap(),
                },
            },
        ];

        let mut mock_loader = MockFileLoader::new();

        // Only Ruby package should be processed
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/ruby/ruby-gem.gemspec"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "packages/ruby/lib/ruby-gem/version.rb",
            ))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/ruby/lib/version.rb"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/ruby/version.rb"))
            .times(1)
            .returning(|_| Ok(None));

        let result = updater.update(packages, &mock_loader).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_with_valid_gemspec() {
        let updater = RubyUpdater::new();
        let package = create_test_updater_package(
            "my-gem",
            "packages/my-gem",
            "3.0.0",
            Framework::Ruby,
        );

        let gemspec_content = r#"
Gem::Specification.new do |spec|
  spec.name = "my-gem"
  spec.version = "1.0.0"
  spec.authors = ["John Doe"]
  spec.email = ["john@example.com"]
  spec.summary = "A test gem"
  spec.description = "A longer description"
  spec.homepage = "https://example.com"
  spec.license = "MIT"

  spec.files = Dir["lib/**/*", "README.md"]
  spec.require_paths = ["lib"]

  spec.add_dependency "rake", "~> 13.0"
end
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/my-gem/my-gem.gemspec"))
            .times(1)
            .returning(move |_| Ok(Some(gemspec_content.to_string())));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq(
                "packages/my-gem/lib/my-gem/version.rb",
            ))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/my-gem/lib/version.rb"))
            .times(1)
            .returning(|_| Ok(None));

        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/my-gem/version.rb"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = updater.update(packages, &mock_loader).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/my-gem/my-gem.gemspec");

        let content = &changes[0].content;
        assert!(content.contains(r#"spec.version = "3.0.0""#));
        assert!(content.contains(r#"spec.name = "my-gem""#));
        assert!(content.contains(r#"spec.license = "MIT""#));
    }
}
