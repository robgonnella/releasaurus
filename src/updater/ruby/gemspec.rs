use log::*;
use regex::Regex;
use std::path::Path;

use crate::{
    forge::request::{FileChange, FileUpdateType},
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
        package: &UpdaterPackage,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes = vec![];

        for manifest in package.manifest_files.iter() {
            let file_path = Path::new(&manifest.file_basename);

            if let Some(file_ext) = file_path.extension() {
                if file_ext.display().to_string() != "gemspec" {
                    continue;
                }

                info!("processing gemspec file: {}", manifest.file_basename);

                let updated_content = self.update_version(
                    &manifest.content,
                    &package.next_version.semver.to_string(),
                );

                if updated_content != manifest.content {
                    file_changes.push(FileChange {
                        path: manifest.file_path.clone(),
                        content: updated_content,
                        update_type: FileUpdateType::Replace,
                    });
                }
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
    use super::*;
    use crate::{
        test_helpers::create_test_tag,
        updater::framework::{Framework, ManifestFile, UpdaterPackage},
    };

    #[tokio::test]
    async fn updates_version_with_spec_prefix_and_double_quotes() {
        let gemspec = Gemspec::new();
        let content = r#"Gem::Specification.new do |spec|
  spec.name = "my-gem"
  spec.version = "1.0.0"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "my-gem.gemspec".to_string(),
            file_basename: "my-gem.gemspec".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = gemspec.process_packages(&package).await.unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("spec.version = \"2.0.0\""));
    }

    #[tokio::test]
    async fn updates_version_with_s_prefix() {
        let gemspec = Gemspec::new();
        let content = r#"Gem::Specification.new do |s|
  s.name = "my-gem"
  s.version = "1.0.0"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "my-gem.gemspec".to_string(),
            file_basename: "my-gem.gemspec".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = gemspec.process_packages(&package).await.unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("s.version = \"2.0.0\""));
    }

    #[tokio::test]
    async fn updates_version_with_single_quotes() {
        let gemspec = Gemspec::new();
        let content = r#"Gem::Specification.new do |spec|
  spec.name = 'my-gem'
  spec.version = '1.0.0'
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "my-gem.gemspec".to_string(),
            file_basename: "my-gem.gemspec".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = gemspec.process_packages(&package).await.unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("spec.version = '2.0.0'"));
    }

    #[tokio::test]
    async fn preserves_whitespace_formatting() {
        let gemspec = Gemspec::new();
        let content = r#"Gem::Specification.new do |spec|
  spec.version   =   "1.0.0"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "my-gem.gemspec".to_string(),
            file_basename: "my-gem.gemspec".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = gemspec.process_packages(&package).await.unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("spec.version   =   \"2.0.0\""));
    }

    #[tokio::test]
    async fn preserves_other_fields() {
        let gemspec = Gemspec::new();
        let content = r#"Gem::Specification.new do |spec|
  spec.name = "my-gem"
  spec.version = "1.0.0"
  spec.authors = ["Test Author"]
  spec.summary = "A test gem"
  spec.files = Dir["lib/**/*"]

  spec.add_dependency "rails", "~> 7.0"
end
"#;
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "my-gem.gemspec".to_string(),
            file_basename: "my-gem.gemspec".to_string(),
            content: content.to_string(),
        };
        let package = UpdaterPackage {
            package_name: "my-gem".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest.clone()],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = gemspec.process_packages(&package).await.unwrap();

        assert!(result.is_some());
        let updated = result.unwrap()[0].content.clone();
        assert!(updated.contains("spec.version = \"2.0.0\""));
        assert!(updated.contains("spec.name = \"my-gem\""));
        assert!(updated.contains("spec.authors = [\"Test Author\"]"));
        assert!(updated.contains("spec.summary = \"A test gem\""));
        assert!(updated.contains("spec.add_dependency \"rails\", \"~> 7.0\""));
    }

    #[tokio::test]
    async fn process_packages_handles_multiple_gemspec_files() {
        let gemspec = Gemspec::new();
        let manifest1 = ManifestFile {
            is_workspace: false,
            file_path: "gems/a/gem-a.gemspec".to_string(),
            file_basename: "gem-a.gemspec".to_string(),
            content: "Gem::Specification.new do |spec|\n  spec.version = \"1.0.0\"\nend\n".to_string(),
        };
        let manifest2 = ManifestFile {
            is_workspace: false,
            file_path: "gems/b/gem-b.gemspec".to_string(),
            file_basename: "gem-b.gemspec".to_string(),
            content: "Gem::Specification.new do |spec|\n  spec.version = \"1.0.0\"\nend\n".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest1, manifest2],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = gemspec.process_packages(&package).await.unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.content.contains("2.0.0")));
    }

    #[tokio::test]
    async fn process_packages_returns_none_when_no_gemspec_files() {
        let gemspec = Gemspec::new();
        let manifest = ManifestFile {
            is_workspace: false,
            file_path: "Gemfile".to_string(),
            file_basename: "Gemfile".to_string(),
            content: "source 'https://rubygems.org'\ngem 'rails'".to_string(),
        };
        let package = UpdaterPackage {
            package_name: "test".to_string(),
            workspace_root: ".".to_string(),
            manifest_files: vec![manifest],
            next_version: create_test_tag("v2.0.0", "2.0.0", "abc"),
            framework: Framework::Ruby,
        };

        let result = gemspec.process_packages(&package).await.unwrap();

        assert!(result.is_none());
    }
}
