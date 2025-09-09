use color_eyre::eyre::eyre;
use std::path::Path;

use crate::{
    result::Result,
    updater::{
        detection::{
            helper::DetectionHelper,
            traits::FrameworkDetector,
            types::{DetectionPattern, FrameworkDetection},
        },
        framework::Framework,
    },
};

pub struct RubyDetector {}

impl RubyDetector {
    pub fn new() -> Self {
        Self {}
    }

    /// Detect Ruby projects with Gemfile (Rails, Sinatra, most Ruby projects)
    fn detect_ruby_bundler(&self, path: &Path) -> Result<FrameworkDetection> {
        let pattern = DetectionPattern {
            manifest_files: vec!["Gemfile"],
            support_files: vec![
                "Gemfile.lock",
                ".ruby-version",
                ".rbenv-version",
                ".rvmrc",
                "lib",
                "spec",
                "test",
                "config.ru",
                "app",
                "config",
                ".rspec",
                "bin",
                "Rakefile",
                "Capfile",
            ],
            content_patterns: vec![
                "gem ",
                "source ",
                "ruby ",
                "gemspec",
                "bundle",
                "Rails.application",
                "Sinatra::",
                "require ",
            ],
            base_confidence: 0.8,
        };

        DetectionHelper::analyze_with_pattern(
            path,
            pattern.clone(),
            |support_evidence| FrameworkDetection {
                framework: Framework::Ruby,
                confidence: DetectionHelper::calculate_confidence(
                    &pattern,
                    &support_evidence,
                ),
                evidence: support_evidence,
            },
        )
    }

    fn find_gemspec(&self, path: &Path) -> Option<String> {
        // Check for gemspec files manually first
        let mut gemspec_filename = None;

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str()
                    && file_name.ends_with(".gemspec")
                {
                    gemspec_filename = Some(file_name.to_string());
                    break;
                }
            }
        }

        gemspec_filename
    }

    /// Detect Ruby gems with gemspec files
    fn detect_ruby_gemspec(&self, path: &Path) -> Result<FrameworkDetection> {
        // Check for gemspec files manually first
        let gemspec = self.find_gemspec(path);

        if gemspec.is_none() {
            return Err(eyre!("No gemspec files found"));
        }

        let gemspec_name = gemspec.unwrap();

        let pattern = DetectionPattern {
            manifest_files: vec![&gemspec_name],
            base_confidence: 0.8,
            support_files: vec![
                "lib",
                "spec",
                "test",
                ".ruby-version",
                ".rbenv-version",
                "bin",
                "Rakefile",
                "Gemfile",
            ],
            content_patterns: vec![
                "Gem::Specification",
                "spec.name",
                "spec.version",
                "spec.authors",
                "spec.summary",
                "spec.add_dependency",
            ],
        };

        DetectionHelper::analyze_with_pattern(
            path,
            pattern.clone(),
            |mut support_evidence| {
                support_evidence.insert(0, "found *.gemspec".to_string());

                FrameworkDetection {
                    framework: Framework::Ruby,
                    confidence: DetectionHelper::calculate_confidence(
                        &pattern,
                        &support_evidence,
                    ),
                    evidence: support_evidence,
                }
            },
        )
    }

    /// Detect Ruby projects with just Rakefile (less common)
    fn detect_ruby_rake(&self, path: &Path) -> Result<FrameworkDetection> {
        let pattern = DetectionPattern {
            manifest_files: vec!["Rakefile"],
            support_files: vec![
                "lib",
                "spec",
                "test",
                ".ruby-version",
                ".rbenv-version",
                "bin",
            ],
            content_patterns: vec![
                "require ",
                "task ",
                "desc ",
                "namespace ",
                "Rake::",
            ],
            base_confidence: 0.6,
        };

        DetectionHelper::analyze_with_pattern(
            path,
            pattern.clone(),
            |support_evidence| FrameworkDetection {
                framework: Framework::Ruby,
                confidence: DetectionHelper::calculate_confidence(
                    &pattern,
                    &support_evidence,
                ),
                evidence: support_evidence,
            },
        )
    }
}

impl FrameworkDetector for RubyDetector {
    fn name(&self) -> &str {
        "ruby"
    }

    fn detect(&self, path: &Path) -> Result<FrameworkDetection> {
        // Try Bundler-based projects first (most common)
        if let Ok(detection) = self.detect_ruby_bundler(path) {
            return Ok(detection);
        }

        // Try gemspec-based projects
        if let Ok(detection) = self.detect_ruby_gemspec(path) {
            return Ok(detection);
        }

        // Fall back to Rake-only projects
        self.detect_ruby_rake(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_ruby_bundler_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create Gemfile
        fs::write(
            path.join("Gemfile"),
            r#"source 'https://rubygems.org'

ruby '3.1.0'

gem 'rails', '~> 7.0.0'
gem 'pg', '~> 1.1'
gem 'puma', '~> 5.0'

group :development, :test do
  gem 'rspec-rails'
end"#,
        )
        .unwrap();

        // Create supporting files
        fs::write(
            path.join("Gemfile.lock"),
            "GEM\n  remote: https://rubygems.org\n  specs:\n",
        )
        .unwrap();
        fs::create_dir_all(path.join("lib")).unwrap();
        fs::write(path.join(".ruby-version"), "3.1.0").unwrap();

        let detector = RubyDetector::new();
        let detection = detector.detect_ruby_bundler(path).unwrap();

        assert!(matches!(detection.framework, Framework::Ruby));
        assert!(detection.confidence > 0.8);
        assert!(detection.evidence.contains(&"found Gemfile".to_string()));
        assert!(
            detection
                .evidence
                .contains(&"found Gemfile.lock".to_string())
        );
    }

    #[test]
    fn test_rails_application_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create Rails-style structure
        fs::write(
            path.join("Gemfile"),
            r#"source 'https://rubygems.org'

gem 'rails', '~> 7.0.0'"#,
        )
        .unwrap();

        fs::create_dir_all(path.join("app/controllers")).unwrap();
        fs::create_dir_all(path.join("app/models")).unwrap();
        fs::create_dir_all(path.join("config")).unwrap();

        fs::write(
            path.join("config.ru"),
            r#"require_relative 'config/environment'

run Rails.application"#,
        )
        .unwrap();

        let detector = RubyDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Ruby));
        assert!(detection.confidence > 0.9);
        assert!(detection.evidence.contains(&"found config.ru".to_string()));
        assert!(detection.evidence.contains(&"found app".to_string()));
        assert!(detection.evidence.contains(&"found config".to_string()));
    }

    #[test]
    fn test_sinatra_application_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create Sinatra application
        fs::write(
            path.join("Gemfile"),
            r#"source 'https://rubygems.org'

gem 'sinatra'
gem 'thin'"#,
        )
        .unwrap();

        fs::write(
            path.join("config.ru"),
            r#"require './app'
run Sinatra::Application"#,
        )
        .unwrap();

        let detector = RubyDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Ruby));
        assert!(detection.confidence > 0.8);
        assert!(detection.evidence.contains(&"found config.ru".to_string()));
    }

    #[test]
    fn test_ruby_with_rspec_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create Ruby project with RSpec
        fs::write(
            path.join("Gemfile"),
            r#"source 'https://rubygems.org'

gem 'rspec'
gem 'rake'"#,
        )
        .unwrap();

        fs::create_dir_all(path.join("spec")).unwrap();
        fs::write(
            path.join(".rspec"),
            r#"--require spec_helper
--color
--format documentation"#,
        )
        .unwrap();

        let detector = RubyDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Ruby));
        assert!(detection.confidence > 0.8);
        assert!(detection.evidence.contains(&"found spec".to_string()));
        assert!(detection.evidence.contains(&"found .rspec".to_string()));
    }

    #[test]
    fn test_rakefile_only_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create Rakefile-only project
        fs::write(
            path.join("Rakefile"),
            r#"require 'rake/testtask'

desc "Run tests"
task :test do
  ruby "test/all_tests.rb"
end

task :default => :test"#,
        )
        .unwrap();

        fs::create_dir_all(path.join("lib")).unwrap();
        fs::create_dir_all(path.join("test")).unwrap();

        let detector = RubyDetector::new();
        let detection = detector.detect_ruby_rake(path).unwrap();

        assert!(matches!(detection.framework, Framework::Ruby));
        assert!(detection.confidence > 0.6);
        assert!(detection.evidence.contains(&"found Rakefile".to_string()));
        assert!(detection.evidence.contains(&"found lib".to_string()));
        assert!(detection.evidence.contains(&"found test".to_string()));
    }

    #[test]
    fn test_detector_name() {
        let detector = RubyDetector::new();
        assert_eq!(detector.name(), "ruby");
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        let detector = RubyDetector::new();
        let result = detector.detect(path);

        // Should return an error for empty directory
        assert!(result.is_err());
    }

    #[test]
    fn test_no_ruby_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create non-Ruby files
        fs::write(path.join("package.json"), "{}").unwrap();
        fs::write(path.join("main.js"), "console.log('hello');").unwrap();
        fs::create_dir_all(path.join("node_modules")).unwrap();

        let detector = RubyDetector::new();
        let result = detector.detect(path);

        // Should return an error since no Ruby files are found
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_prefers_bundler_over_gem() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create both Gemfile and gemspec
        fs::write(
            path.join("Gemfile"),
            r#"source 'https://rubygems.org'
gemspec"#,
        )
        .unwrap();

        fs::write(
            path.join("test.gemspec"),
            r#"Gem::Specification.new do |spec|
  spec.name = "test"
  spec.version = "0.1.0"
end"#,
        )
        .unwrap();

        let detector = RubyDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Ruby));
        // Should prefer Bundler detection (higher confidence)
        assert!(detection.confidence > 0.8);
        assert!(detection.evidence.contains(&"found Gemfile".to_string()));
    }

    #[test]
    fn test_comprehensive_ruby_detection_with_all_evidence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create comprehensive Ruby project with multiple evidence types
        fs::write(
            path.join("Gemfile"),
            r#"source 'https://rubygems.org'

ruby '3.1.0'

gem 'rails', '~> 7.0.0'
gem 'rspec-rails'
gemspec"#,
        )
        .unwrap();

        fs::write(path.join("Gemfile.lock"), "GEM\n  specs:\n").unwrap();
        fs::write(path.join(".ruby-version"), "3.1.0").unwrap();
        fs::write(path.join("Rakefile"), "require 'rake'").unwrap();
        fs::write(path.join("Capfile"), "require 'capistrano'").unwrap();

        // Create directories
        fs::create_dir_all(path.join("app/controllers")).unwrap();
        fs::create_dir_all(path.join("config")).unwrap();
        fs::create_dir_all(path.join("spec")).unwrap();
        fs::create_dir_all(path.join("lib")).unwrap();
        fs::create_dir_all(path.join("bin")).unwrap();

        fs::write(
            path.join("config.ru"),
            r#"require_relative 'config/environment'
run Rails.application"#,
        )
        .unwrap();

        fs::write(path.join(".rspec"), "--require spec_helper").unwrap();

        let detector = RubyDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Ruby));
        assert!(detection.confidence > 0.9);

        // Check for multiple evidence types
        let evidence = &detection.evidence;
        assert!(evidence.contains(&"found Gemfile".to_string()));
        assert!(evidence.contains(&"found Gemfile.lock".to_string()));
        assert!(evidence.contains(&"found .ruby-version".to_string()));
        assert!(evidence.contains(&"found Rakefile".to_string()));
        assert!(evidence.contains(&"found config.ru".to_string()));
        assert!(evidence.contains(&"found app".to_string()));
        assert!(evidence.contains(&"found config".to_string()));
        assert!(evidence.contains(&"found spec".to_string()));
        assert!(evidence.contains(&"found lib".to_string()));
        assert!(evidence.contains(&"found .rspec".to_string()));
        assert!(evidence.contains(&"found Capfile".to_string()));
        assert!(evidence.contains(&"found bin".to_string()));
    }
}
