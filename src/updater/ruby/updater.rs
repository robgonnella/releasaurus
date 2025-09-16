use log::*;
use regex::Regex;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

use crate::{
    result::Result,
    updater::{
        framework::Framework, framework::Package, traits::PackageUpdater,
    },
};

/// Ruby package updater for Gem and Bundler projects.
pub struct RubyUpdater {}

impl RubyUpdater {
    pub fn new() -> Self {
        Self {}
    }

    /// Process packages and update their version files
    fn process_packages(&self, packages: &[Package]) -> Result<()> {
        for package in packages {
            let package_path = Path::new(&package.path);
            info!("Updating Ruby package: {}", package.path);

            // Update gemspec files first (primary version source)
            self.update_gemspec_files(package_path, package)?;

            // Update version.rb files (secondary version source)
            self.update_version_files(package_path, package)?;
        }

        Ok(())
    }

    /// Update gemspec files in the package directory
    fn update_gemspec_files(
        &self,
        package_path: &Path,
        package: &Package,
    ) -> Result<()> {
        // Look for gemspec files
        if let Ok(entries) = std::fs::read_dir(package_path) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str()
                    && file_name.ends_with(".gemspec")
                {
                    info!("Updating gemspec file: {}", entry.path().display());
                    self.update_gemspec_file(&entry.path(), package)?;
                }
            }
        }

        Ok(())
    }

    /// Update a single gemspec file
    fn update_gemspec_file(
        &self,
        gemspec_path: &Path,
        package: &Package,
    ) -> Result<()> {
        let mut file = File::open(gemspec_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let new_version = package.next_version.semver.to_string();

        // Define regex patterns for different gemspec version declaration styles
        let patterns = vec![
            // spec.version = "1.0.0"
            Regex::new(r#"(\s*spec\.version\s*=\s*)["'][^"']*["']"#)?,
            // s.version = "1.0.0"
            Regex::new(r#"(\s*s\.version\s*=\s*)["'][^"']*["']"#)?,
            // gem.version = "1.0.0"
            Regex::new(r#"(\s*gem\.version\s*=\s*)["'][^"']*["']"#)?,
            // version "1.0.0" (method style)
            Regex::new(r#"(\s*version\s+)["'][^"']*["']"#)?,
        ];

        let mut updated_content = content.clone();
        let mut version_found = false;

        for pattern in patterns {
            if pattern.is_match(&content) {
                updated_content = pattern
                    .replace_all(&updated_content, |caps: &regex::Captures| {
                        let prefix = caps.get(1).unwrap().as_str();
                        // Preserve the original quote style
                        let quote_char = if prefix.contains('"')
                            || content[caps.get(0).unwrap().range()]
                                .contains('"')
                        {
                            '"'
                        } else {
                            '\''
                        };
                        format!(
                            "{}{}{}{}",
                            prefix, quote_char, new_version, quote_char
                        )
                    })
                    .to_string();
                version_found = true;
                break;
            }
        }

        if version_found {
            info!("Updating gemspec version to: {}", new_version);
            let mut output_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(gemspec_path)?;
            output_file.write_all(updated_content.as_bytes())?;
        } else {
            info!(
                "No version declaration found in gemspec: {}",
                gemspec_path.display()
            );
        }

        Ok(())
    }

    /// Update version.rb files in the package
    fn update_version_files(
        &self,
        package_path: &Path,
        package: &Package,
    ) -> Result<()> {
        // Look for version.rb files in lib/ directory
        let lib_path = package_path.join("lib");
        if lib_path.exists() {
            self.scan_for_version_files(&lib_path, package)?;
        }

        Ok(())
    }

    /// Recursively scan for version.rb files
    fn scan_for_version_files(
        &self,
        dir_path: &Path,
        package: &Package,
    ) -> Result<()> {
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Recursively scan subdirectories
                    self.scan_for_version_files(&path, package)?;
                } else if let Some(file_name) = path.file_name()
                    && file_name == "version.rb"
                {
                    info!("Updating version file: {}", path.display());
                    self.update_version_file(&path, package)?;
                }
            }
        }

        Ok(())
    }

    /// Update a single version.rb file
    fn update_version_file(
        &self,
        version_file_path: &Path,
        package: &Package,
    ) -> Result<()> {
        let file = File::open(version_file_path)?;
        let reader = BufReader::new(file);
        let mut lines: Vec<String> = Vec::new();
        let mut version_updated = false;

        let new_version = package.next_version.semver.to_string();

        // Define regex patterns for Ruby version constants
        let patterns = vec![
            // VERSION = "1.0.0"
            Regex::new(r#"^(\s*VERSION\s*=\s*)["'][^"']*["'](.*)$"#)?,
            // ::VERSION = "1.0.0"
            Regex::new(r#"^(\s*::VERSION\s*=\s*)["'][^"']*["'](.*)$"#)?,
            // Module::VERSION = "1.0.0"
            Regex::new(r#"^(\s*\w+::VERSION\s*=\s*)["'][^"']*["'](.*)$"#)?,
        ];

        // Read all lines and update version constants
        for line in reader.lines() {
            let line = line?;
            let mut line_updated = false;

            for pattern in &patterns {
                if let Some(captures) = pattern.captures(&line) {
                    let prefix = captures.get(1).unwrap().as_str();
                    let suffix = captures.get(2).map_or("", |m| m.as_str());

                    // Preserve the original quote style
                    let quote_char =
                        if line.contains('"') { '"' } else { '\'' };

                    let updated_line = format!(
                        "{}{}{}{}{}",
                        prefix, quote_char, new_version, quote_char, suffix
                    );

                    lines.push(updated_line);
                    version_updated = true;
                    line_updated = true;
                    info!(
                        "Updated version constant in {}: {}",
                        version_file_path.display(),
                        new_version
                    );
                    break;
                }
            }

            if !line_updated {
                lines.push(line);
            }
        }

        // Only write back if we actually updated something
        if version_updated {
            let mut output_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(version_file_path)?;

            for line in lines {
                writeln!(output_file, "{}", line)?;
            }
        }

        Ok(())
    }
}

impl PackageUpdater for RubyUpdater {
    fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()> {
        let ruby_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Ruby))
            .collect::<Vec<Package>>();

        info!(
            "Found {} Ruby packages in {}",
            ruby_packages.len(),
            root_path.display(),
        );

        self.process_packages(&ruby_packages)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyzer::types::Tag, updater::framework::Framework};
    use std::fs;
    use tempfile::TempDir;

    fn create_test_package(
        name: &str,
        path: &str,
        version: &str,
        framework: Framework,
    ) -> Package {
        Package::new(
            name.to_string(),
            path.to_string(),
            Tag {
                sha: "abc123".into(),
                name: format!("v{}", version),
                semver: semver::Version::parse(version).unwrap(),
            },
            framework,
        )
    }

    #[test]
    fn test_ruby_updater_creation() {
        let _updater = RubyUpdater::new();
        // Basic test to ensure the updater can be created without panicking
    }

    #[test]
    fn test_ruby_updater_empty_packages() {
        let updater = RubyUpdater::new();
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let packages = vec![];

        let result = updater.update(path, packages);
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_filters_ruby_packages_only() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create Ruby package
        let ruby_dir = root_path.join("ruby-gem");
        fs::create_dir_all(&ruby_dir).unwrap();
        fs::write(
            ruby_dir.join("test.gemspec"),
            r#"Gem::Specification.new do |spec|
  spec.name = "test"
  spec.version = "1.0.0"
  spec.summary = "A test gem"
end"#,
        )
        .unwrap();

        // Create non-Ruby package
        let node_dir = root_path.join("node-package");
        fs::create_dir_all(&node_dir).unwrap();

        let packages = vec![
            create_test_package(
                "test",
                ruby_dir.to_str().unwrap(),
                "2.0.0",
                Framework::Ruby,
            ),
            create_test_package(
                "node-package",
                node_dir.to_str().unwrap(),
                "2.0.0",
                Framework::Node,
            ),
        ];

        let updater = RubyUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Check that Ruby package was updated
        let updated_content =
            fs::read_to_string(ruby_dir.join("test.gemspec")).unwrap();
        assert!(updated_content.contains("spec.version = \"2.0.0\""));
    }

    #[test]
    fn test_update_gemspec_file() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("ruby-gem");
        fs::create_dir_all(&package_dir).unwrap();

        // Create initial gemspec
        fs::write(
            package_dir.join("my_gem.gemspec"),
            r#"Gem::Specification.new do |spec|
  spec.name = "my_gem"
  spec.version = "1.0.0"
  spec.authors = ["Developer"]
  spec.email = ["dev@example.com"]
  spec.summary = "A Ruby gem"
  spec.description = "A longer description"
  spec.homepage = "https://github.com/user/my_gem"
  spec.license = "MIT"

  spec.files = Dir.chdir(File.expand_path('..', __FILE__)) do
    `git ls-files -z`.split("\x0")
  end
  spec.require_paths = ["lib"]
end"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "my_gem",
            package_dir.to_str().unwrap(),
            "2.1.0",
            Framework::Ruby,
        )];

        let updater = RubyUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Verify the version was updated
        let updated_content =
            fs::read_to_string(package_dir.join("my_gem.gemspec")).unwrap();
        assert!(updated_content.contains("spec.version = \"2.1.0\""));

        // Verify other fields remain unchanged
        assert!(updated_content.contains("spec.name = \"my_gem\""));
        assert!(updated_content.contains("spec.summary = \"A Ruby gem\""));
        assert!(updated_content.contains("spec.license = \"MIT\""));
    }

    #[test]
    fn test_update_version_rb_file() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("ruby-gem");
        fs::create_dir_all(package_dir.join("lib/my_gem")).unwrap();

        // Create gemspec
        fs::write(
            package_dir.join("my_gem.gemspec"),
            r#"Gem::Specification.new do |spec|
  spec.name = "my_gem"
  spec.version = "1.0.0"
end"#,
        )
        .unwrap();

        // Create version.rb file
        fs::write(
            package_dir.join("lib/my_gem/version.rb"),
            r#"module MyGem
  VERSION = "1.0.0"
end"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "my_gem",
            package_dir.to_str().unwrap(),
            "2.1.0",
            Framework::Ruby,
        )];

        let updater = RubyUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Verify both files were updated
        let gemspec_content =
            fs::read_to_string(package_dir.join("my_gem.gemspec")).unwrap();
        assert!(gemspec_content.contains("spec.version = \"2.1.0\""));

        let version_content =
            fs::read_to_string(package_dir.join("lib/my_gem/version.rb"))
                .unwrap();
        assert!(version_content.contains("VERSION = \"2.1.0\""));
        assert!(version_content.contains("module MyGem"));
    }

    #[test]
    fn test_update_multiple_ruby_packages() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create first Ruby gem
        let gem1_dir = root_path.join("gem1");
        fs::create_dir_all(&gem1_dir).unwrap();
        fs::write(
            gem1_dir.join("gem1.gemspec"),
            r#"Gem::Specification.new do |s|
  s.name = "gem1"
  s.version = "1.0.0"
end"#,
        )
        .unwrap();

        // Create second Ruby gem
        let gem2_dir = root_path.join("gem2");
        fs::create_dir_all(&gem2_dir).unwrap();
        fs::write(
            gem2_dir.join("gem2.gemspec"),
            r#"Gem::Specification.new do |gem|
  gem.name = "gem2"
  gem.version = "0.5.0"
end"#,
        )
        .unwrap();

        let packages = vec![
            create_test_package(
                "gem1",
                gem1_dir.to_str().unwrap(),
                "1.1.0",
                Framework::Ruby,
            ),
            create_test_package(
                "gem2",
                gem2_dir.to_str().unwrap(),
                "0.6.0",
                Framework::Ruby,
            ),
        ];

        let updater = RubyUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Verify both packages were updated
        let gem1_content =
            fs::read_to_string(gem1_dir.join("gem1.gemspec")).unwrap();
        assert!(gem1_content.contains("s.version = \"1.1.0\""));

        let gem2_content =
            fs::read_to_string(gem2_dir.join("gem2.gemspec")).unwrap();
        assert!(gem2_content.contains("gem.version = \"0.6.0\""));
    }

    #[test]
    fn test_gemspec_version_variations() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Test different gemspec version declaration styles
        let test_cases = vec![
            ("spec.version = '1.0.0'", "spec.version = '2.0.0'"),
            ("spec.version = \"1.0.0\"", "spec.version = \"2.0.0\""),
            ("s.version = '1.0.0'", "s.version = '2.0.0'"),
            ("gem.version = \"1.0.0\"", "gem.version = \"2.0.0\""),
            ("  version '1.0.0'", "  version '2.0.0'"),
        ];

        for (i, (original, expected)) in test_cases.into_iter().enumerate() {
            let package_dir = root_path.join(format!("gem-test-{}", i));
            fs::create_dir_all(&package_dir).unwrap();

            let gemspec_content = format!(
                r#"Gem::Specification.new do |spec|
  spec.name = "test-gem-{}"
  {}
  spec.summary = "Test gem"
end"#,
                i, original
            );

            fs::write(
                package_dir.join(format!("test-gem-{}.gemspec", i)),
                gemspec_content,
            )
            .unwrap();

            let packages = vec![create_test_package(
                format!("test-gem-{}", i).as_str(),
                package_dir.to_str().unwrap(),
                "2.0.0",
                Framework::Ruby,
            )];

            let updater = RubyUpdater::new();
            let result = updater.update(root_path, packages);
            assert!(result.is_ok());

            let updated_content = fs::read_to_string(
                package_dir.join(format!("test-gem-{}.gemspec", i)),
            )
            .unwrap();
            assert!(
                updated_content.contains(expected),
                "Expected '{}' but got content: {}",
                expected,
                updated_content
            );
        }
    }

    #[test]
    fn test_version_constant_variations() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Test different version constant styles
        let test_cases = vec![
            ("  VERSION = '1.0.0'", "  VERSION = '2.0.0'"),
            ("VERSION = \"1.0.0\"", "VERSION = \"2.0.0\""),
            ("  ::VERSION = '1.0.0'", "  ::VERSION = '2.0.0'"),
            ("MyGem::VERSION = \"1.0.0\"", "MyGem::VERSION = \"2.0.0\""),
        ];

        for (i, (original, expected)) in test_cases.into_iter().enumerate() {
            let package_dir = root_path.join(format!("version-test-{}", i));
            fs::create_dir_all(package_dir.join("lib")).unwrap();

            // Create gemspec
            fs::write(
                package_dir.join("test.gemspec"),
                "Gem::Specification.new do |s|\n  s.name = 'test'\n  s.version = '1.0.0'\nend",
            )
            .unwrap();

            // Create version.rb with different constant styles
            fs::write(
                package_dir.join("lib/version.rb"),
                format!("module Test\n{}\nend", original),
            )
            .unwrap();

            let packages = vec![create_test_package(
                format!("version-test-{}", i).as_str(),
                package_dir.to_str().unwrap(),
                "2.0.0",
                Framework::Ruby,
            )];

            let updater = RubyUpdater::new();
            let result = updater.update(root_path, packages);
            assert!(result.is_ok());

            let updated_content =
                fs::read_to_string(package_dir.join("lib/version.rb")).unwrap();
            assert!(
                updated_content.contains(expected),
                "Expected '{}' but got content: {}",
                expected,
                updated_content
            );
        }
    }

    #[test]
    fn test_update_with_missing_files() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create package directory without Ruby files
        let package_dir = root_path.join("no-ruby-files");
        fs::create_dir_all(&package_dir).unwrap();

        let packages = vec![create_test_package(
            "no-ruby-files",
            package_dir.to_str().unwrap(),
            "1.0.0",
            Framework::Ruby,
        )];

        let updater = RubyUpdater::new();
        let result = updater.update(root_path, packages);
        // Should succeed but skip the package
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_version_file_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("nested-gem");

        // Create nested directory structure
        fs::create_dir_all(package_dir.join("lib/nested_gem/deep")).unwrap();

        // Create gemspec
        fs::write(
            package_dir.join("nested_gem.gemspec"),
            r#"Gem::Specification.new do |spec|
  spec.name = "nested_gem"
  spec.version = "1.0.0"
end"#,
        )
        .unwrap();

        // Create deeply nested version.rb
        fs::write(
            package_dir.join("lib/nested_gem/deep/version.rb"),
            r#"module NestedGem
  module Deep
    VERSION = "1.0.0"
  end
end"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "nested_gem",
            package_dir.to_str().unwrap(),
            "3.0.0",
            Framework::Ruby,
        )];

        let updater = RubyUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Verify nested version file was updated
        let version_content = fs::read_to_string(
            package_dir.join("lib/nested_gem/deep/version.rb"),
        )
        .unwrap();
        assert!(version_content.contains("VERSION = \"3.0.0\""));
    }

    #[test]
    fn test_preserve_file_structure() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("structured-gem");
        fs::create_dir_all(package_dir.join("lib/structured_gem")).unwrap();

        // Create complex gemspec
        fs::write(
            package_dir.join("structured_gem.gemspec"),
            r#"# -*- encoding: utf-8 -*-
lib = File.expand_path('../lib', __FILE__)
$LOAD_PATH.unshift(lib) unless $LOAD_PATH.include?(lib)

Gem::Specification.new do |spec|
  spec.name = "structured_gem"
  spec.version = "1.0.0"
  spec.authors = ["Developer"]
  spec.email = ["dev@example.com"]
  spec.summary = "A structured gem"
  spec.description = "A longer description of the gem"
  spec.homepage = "https://github.com/user/structured_gem"
  spec.license = "MIT"

  spec.files = `git ls-files`.split($/)
  spec.executables = spec.files.grep(%r{^bin/}) { |f| File.basename(f) }
  spec.test_files = spec.files.grep(%r{^(test|spec|features)/})
  spec.require_paths = ["lib"]

  spec.add_dependency "activesupport", ">= 4.0"
  spec.add_development_dependency "rspec", "~> 3.0"
end"#,
        )
        .unwrap();

        // Create structured version file
        fs::write(
            package_dir.join("lib/structured_gem/version.rb"),
            r#"# frozen_string_literal: true

module StructuredGem
  # Current version of the gem
  VERSION = "1.0.0"

  # Version info
  def self.version_info
    "StructuredGem v#{VERSION}"
  end
end"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "structured_gem",
            package_dir.to_str().unwrap(),
            "1.2.3",
            Framework::Ruby,
        )];

        let updater = RubyUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        let gemspec_content =
            fs::read_to_string(package_dir.join("structured_gem.gemspec"))
                .unwrap();

        // Verify version was updated in gemspec
        assert!(gemspec_content.contains("spec.version = \"1.2.3\""));

        // Verify structure is preserved
        assert!(gemspec_content.contains("spec.name = \"structured_gem\""));
        assert!(gemspec_content.contains("spec.authors = [\"Developer\"]"));
        assert!(
            gemspec_content.contains("spec.add_dependency \"activesupport\"")
        );
        assert!(gemspec_content.contains("# -*- encoding: utf-8 -*-"));

        let version_content = fs::read_to_string(
            package_dir.join("lib/structured_gem/version.rb"),
        )
        .unwrap();

        // Verify version was updated in version.rb
        assert!(version_content.contains("VERSION = \"1.2.3\""));

        // Verify structure is preserved
        assert!(version_content.contains("# frozen_string_literal: true"));
        assert!(version_content.contains("module StructuredGem"));
        assert!(version_content.contains("# Current version of the gem"));
        assert!(version_content.contains("def self.version_info"));
    }
}
