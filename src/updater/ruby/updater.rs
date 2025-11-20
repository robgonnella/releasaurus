use async_trait::async_trait;

use crate::{
    forge::request::FileChange,
    result::Result,
    updater::{
        framework::UpdaterPackage,
        ruby::{gemspec::Gemspec, version_rb::VersionRb},
        traits::PackageUpdater,
    },
};

/// Ruby package updater for Gem and Bundler projects.
pub struct RubyUpdater {
    gemspec: Gemspec,
    version_rb: VersionRb,
}

impl RubyUpdater {
    /// Create Ruby updater for Gem and Bundler projects.
    pub fn new() -> Self {
        Self {
            gemspec: Gemspec::new(),
            version_rb: VersionRb::new(),
        }
    }
}

#[async_trait]
impl PackageUpdater for RubyUpdater {
    async fn update(
        &self,
        package: &UpdaterPackage,
        // workspaces not supported for ruby projects
        _workspace_packages: Vec<UpdaterPackage>,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        // Try to update gemspec files
        if let Some(changes) = self.gemspec.process_packages(package).await? {
            file_changes.extend(changes);
        }

        // Try to update version.rb files
        if let Some(changes) = self.version_rb.process_package(package).await? {
            file_changes.extend(changes);
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::analyzer::release::Tag;
//     use crate::forge::traits::MockForge;
//     use crate::test_helpers::create_test_updater_package;
//     use crate::updater::framework::Framework;
//     use semver::Version as SemVer;

//     #[tokio::test]
//     async fn test_update_processes_both_files() {
//         let updater = RubyUpdater::new();
//         let package = create_test_updater_package(
//             "my-gem",
//             "packages/my-gem",
//             "2.0.0",
//             Framework::Ruby,
//         );

//         let gemspec_content = r#"
// Gem::Specification.new do |spec|
//   spec.name = "my-gem"
//   spec.version = "1.0.0"
//   spec.summary = "A test gem"
// end
// "#;

//         let version_content = r#"
// module MyGem
//   VERSION = "1.0.0"
// end
// "#;

//         let mut mock_forge = MockForge::new();

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/my-gem/my-gem.gemspec"))
//             .times(1)
//             .returning({
//                 let content = gemspec_content.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq(
//                 "packages/my-gem/lib/my-gem/version.rb",
//             ))
//             .times(1)
//             .returning({
//                 let content = version_content.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         let packages = vec![package];
//         let result = updater.update(packages).await.unwrap();

//         assert!(result.is_some());
//         let changes = result.unwrap();
//         assert_eq!(changes.len(), 2);

//         // Check gemspec was updated
//         let gemspec_change = changes
//             .iter()
//             .find(|c| c.path.contains(".gemspec"))
//             .unwrap();
//         assert!(gemspec_change.content.contains(r#"spec.version = "2.0.0""#));

//         // Check version.rb was updated
//         let version_change = changes
//             .iter()
//             .find(|c| c.path.contains("version.rb"))
//             .unwrap();
//         assert!(version_change.content.contains(r#"VERSION = "2.0.0""#));
//     }

//     #[tokio::test]
//     async fn test_update_multiple_packages() {
//         let updater = RubyUpdater::new();
//         let packages = vec![
//             create_test_updater_package(
//                 "gem-one",
//                 "packages/one",
//                 "2.0.0",
//                 Framework::Ruby,
//             ),
//             create_test_updater_package(
//                 "gem-two",
//                 "packages/two",
//                 "3.0.0",
//                 Framework::Ruby,
//             ),
//         ];

//         let gemspec1 = r#"
// Gem::Specification.new do |spec|
//   spec.name = "gem-one"
//   spec.version = "1.0.0"
// end
// "#;

//         let gemspec2 = r#"
// Gem::Specification.new do |spec|
//   spec.name = "gem-two"
//   spec.version = "1.0.0"
// end
// "#;

//         let mut mock_forge = MockForge::new();

//         // Package one
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/one/gem-one.gemspec"))
//             .times(1)
//             .returning({
//                 let content = gemspec1.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq(
//                 "packages/one/lib/gem-one/version.rb",
//             ))
//             .times(1)
//             .returning(|_| Ok(None));

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/one/lib/version.rb"))
//             .times(1)
//             .returning(|_| Ok(None));

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/one/version.rb"))
//             .times(1)
//             .returning(|_| Ok(None));

//         // Package two
//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/two/gem-two.gemspec"))
//             .times(1)
//             .returning({
//                 let content = gemspec2.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq(
//                 "packages/two/lib/gem-two/version.rb",
//             ))
//             .times(1)
//             .returning(|_| Ok(None));

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/two/lib/version.rb"))
//             .times(1)
//             .returning(|_| Ok(None));

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/two/version.rb"))
//             .times(1)
//             .returning(|_| Ok(None));

//         let result = updater.update(packages).await.unwrap();

//         assert!(result.is_some());
//         let changes = result.unwrap();
//         assert_eq!(changes.len(), 2);

//         // Check first package
//         let change1 = changes
//             .iter()
//             .find(|c| c.path == "packages/one/gem-one.gemspec")
//             .unwrap();
//         assert!(change1.content.contains(r#"spec.version = "2.0.0""#));

//         // Check second package
//         let change2 = changes
//             .iter()
//             .find(|c| c.path == "packages/two/gem-two.gemspec")
//             .unwrap();
//         assert!(change2.content.contains(r#"spec.version = "3.0.0""#));
//     }

//     #[tokio::test]
//     async fn test_update_no_files_found() {
//         let updater = RubyUpdater::new();
//         let package = create_test_updater_package(
//             "my-gem",
//             "packages/my-gem",
//             "2.0.0",
//             Framework::Ruby,
//         );

//         let mut mock_forge = MockForge::new();

//         mock_forge.expect_get_file_content().returning(|_| Ok(None));

//         let packages = vec![package];
//         let result = updater.update(packages).await.unwrap();

//         assert!(result.is_none());
//     }

//     #[tokio::test]
//     async fn test_update_filters_ruby_packages() {
//         let updater = RubyUpdater::new();

//         let packages = vec![
//             create_test_updater_package(
//                 "ruby-gem",
//                 "packages/ruby",
//                 "2.0.0",
//                 Framework::Ruby,
//             ),
//             UpdaterPackage {
//                 name: "node-package".into(),
//                 path: "packages/node".into(),
//                 workspace_root: ".".into(),
//                 framework: Framework::Node,
//                 next_version: Tag {
//                     sha: "test-sha".into(),
//                     name: "v1.0.0".into(),
//                     semver: SemVer::parse("1.0.0").unwrap(),
//                 },
//             },
//         ];

//         let mut mock_forge = MockForge::new();

//         mock_forge.expect_get_file_content().returning(|_| Ok(None));

//         let result = updater.update(packages).await.unwrap();

//         // Should return None when no Ruby files are found
//         assert!(result.is_none());
//     }

//     #[tokio::test]
//     async fn test_update_with_valid_gemspec() {
//         let updater = RubyUpdater::new();
//         let package = create_test_updater_package(
//             "test-gem",
//             "packages/test",
//             "3.0.0",
//             Framework::Ruby,
//         );

//         let gemspec_content = r#"
// Gem::Specification.new do |spec|
//   spec.name        = "test-gem"
//   spec.version     = "1.0.0"
//   spec.summary     = "Test gem"
//   spec.description = "A test gem for testing"
//   spec.authors     = ["Test Author"]
//   spec.email       = "test@example.com"
//   spec.files       = Dir["lib/**/*.rb"]
//   spec.homepage    = "https://example.com/test-gem"
//   spec.license     = "MIT"
// end
// "#;

//         let mut mock_forge = MockForge::new();

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/test/test-gem.gemspec"))
//             .times(1)
//             .returning({
//                 let content = gemspec_content.to_string();
//                 move |_| Ok(Some(content.clone()))
//             });

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq(
//                 "packages/test/lib/test-gem/version.rb",
//             ))
//             .times(1)
//             .returning(|_| Ok(None));

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/test/lib/version.rb"))
//             .times(1)
//             .returning(|_| Ok(None));

//         mock_forge
//             .expect_get_file_content()
//             .with(mockall::predicate::eq("packages/test/version.rb"))
//             .times(1)
//             .returning(|_| Ok(None));

//         let packages = vec![package];
//         let result = updater.update(packages).await.unwrap();

//         assert!(result.is_some());
//         let changes = result.unwrap();
//         assert_eq!(changes.len(), 1);

//         let content = &changes[0].content;
//         // Verify version is updated
//         assert!(content.contains(r#"spec.version     = "3.0.0""#));

//         // Verify all other fields are preserved
//         assert!(content.contains(r#"spec.name        = "test-gem""#));
//         assert!(content.contains(r#"spec.summary     = "Test gem""#));
//         assert!(content.contains("Test Author"));
//         assert!(content.contains("MIT"));
//     }
// }
