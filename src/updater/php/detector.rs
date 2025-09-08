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

pub struct PhpDetector {}

impl PhpDetector {
    pub fn new() -> Self {
        Self {}
    }
}

impl FrameworkDetector for PhpDetector {
    fn name(&self) -> &str {
        "php"
    }

    fn detect(&self, path: &Path) -> Result<FrameworkDetection> {
        let pattern = DetectionPattern {
            manifest_files: vec!["composer.json"],
            support_files: vec![
                "composer.lock",
                "vendor",
                "autoload.php",
                "index.php",
            ],
            content_patterns: vec![
                "\"name\":",
                "\"version\":",
                "\"require\":",
                "\"autoload\":",
                "\"psr-4\":",
            ],
            base_confidence: 0.8,
        };

        DetectionHelper::analyze_with_pattern(
            path,
            pattern.clone(),
            |support_evidence| FrameworkDetection {
                framework: Framework::Php,
                confidence: DetectionHelper::calculate_confidence(
                    &pattern,
                    &support_evidence,
                ),
                evidence: support_evidence,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_php_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create composer.json
        fs::write(
            path.join("composer.json"),
            r#"{
    "name": "vendor/test-package",
    "version": "1.0.0",
    "type": "library",
    "require": {
        "php": ">=8.0",
        "symfony/console": "^6.0"
    },
    "autoload": {
        "psr-4": {
            "Vendor\\TestPackage\\": "src/"
        }
    }
}"#,
        )
        .unwrap();

        // Create supporting files
        fs::write(path.join("composer.lock"), "{}").unwrap();
        fs::create_dir_all(path.join("vendor")).unwrap();

        let detector = PhpDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Php));
        assert!(detection.confidence > 0.7);
        assert!(
            detection
                .evidence
                .contains(&"found composer.json".to_string())
        );
    }

    #[test]
    fn test_php_detection_with_all_evidence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create comprehensive PHP project structure
        fs::write(
            path.join("composer.json"),
            r#"{
    "name": "example/comprehensive-app",
    "version": "2.1.0",
    "type": "project",
    "require": {
        "php": ">=8.1",
        "laravel/framework": "^10.0",
        "guzzlehttp/guzzle": "^7.2"
    },
    "require-dev": {
        "phpunit/phpunit": "^10.0"
    },
    "autoload": {
        "psr-4": {
            "App\\": "src/",
            "Database\\Seeders\\": "database/seeders/"
        }
    },
    "scripts": {
        "test": "phpunit",
        "post-install-cmd": [
            "@php artisan optimize"
        ]
    }
}"#,
        )
        .unwrap();

        // Create all supporting files and directories
        fs::write(
            path.join("composer.lock"),
            r#"{
    "_readme": [
        "This file locks the dependencies of your project to a known state"
    ],
    "content-hash": "abc123",
    "packages": []
}"#,
        )
        .unwrap();

        fs::create_dir_all(path.join("vendor")).unwrap();
        fs::write(path.join("index.php"), "<?php\necho 'Hello World';")
            .unwrap();

        let detector = PhpDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Php));
        // Should have very high confidence with all evidence present
        assert!(detection.confidence > 0.9);

        // Check that multiple pieces of evidence were found
        assert!(
            detection
                .evidence
                .contains(&"found composer.json".to_string())
        );
        assert!(
            detection
                .evidence
                .contains(&"found composer.lock".to_string())
        );
        assert!(detection.evidence.contains(&"found vendor".to_string()));
        assert!(detection.evidence.contains(&"found index.php".to_string()));
        assert!(
            detection
                .evidence
                .contains(&"contains \"name\":".to_string())
        );
        assert!(
            detection
                .evidence
                .contains(&"contains \"version\":".to_string())
        );
        assert!(
            detection
                .evidence
                .contains(&"contains \"require\":".to_string())
        );
        assert!(
            detection
                .evidence
                .contains(&"contains \"autoload\":".to_string())
        );
        assert!(
            detection
                .evidence
                .contains(&"contains \"psr-4\":".to_string())
        );
    }

    #[test]
    fn test_laravel_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("composer.json"),
            r#"{
    "name": "laravel/laravel",
    "type": "project",
    "require": {
        "php": "^8.1",
        "laravel/framework": "^10.10",
        "laravel/sanctum": "^3.2",
        "laravel/tinker": "^2.8"
    },
    "autoload": {
        "psr-4": {
            "App\\": "app/",
            "Database\\Factories\\": "database/factories/",
            "Database\\Seeders\\": "database/seeders/"
        }
    }
}"#,
        )
        .unwrap();

        fs::create_dir_all(path.join("vendor")).unwrap();
        fs::write(path.join("composer.lock"), "{}").unwrap();

        let detector = PhpDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Php));
        assert!(detection.confidence > 0.8);
        assert!(
            detection
                .evidence
                .contains(&"contains \"autoload\":".to_string())
        );
    }

    #[test]
    fn test_symfony_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("composer.json"),
            r#"{
    "name": "symfony/skeleton",
    "type": "project",
    "license": "MIT",
    "require": {
        "php": ">=8.1",
        "ext-ctype": "*",
        "ext-iconv": "*",
        "symfony/console": "6.3.*",
        "symfony/dotenv": "6.3.*",
        "symfony/flex": "^2",
        "symfony/framework-bundle": "6.3.*",
        "symfony/runtime": "6.3.*",
        "symfony/yaml": "6.3.*"
    },
    "autoload": {
        "psr-4": {
            "App\\": "src/"
        }
    }
}"#,
        )
        .unwrap();

        fs::create_dir_all(path.join("src")).unwrap();
        fs::write(path.join("composer.lock"), "{}").unwrap();

        let detector = PhpDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Php));
        assert!(detection.confidence > 0.8);
    }

    #[test]
    fn test_minimal_php_package() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("composer.json"),
            r#"{
    "name": "vendor/minimal-package",
    "require": {
        "php": ">=7.4"
    }
}"#,
        )
        .unwrap();

        let detector = PhpDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Php));
        // Lower confidence without version and autoload, but still above base due to "require"
        assert!(detection.confidence > 0.8);
        assert!(detection.confidence < 0.91);
    }

    #[test]
    fn test_php_library_with_tests() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("composer.json"),
            r#"{
    "name": "monolog/monolog",
    "version": "3.4.0",
    "type": "library",
    "require": {
        "php": ">=8.1",
        "psr/log": "^2.0 || ^3.0"
    },
    "require-dev": {
        "aws/aws-sdk-php": "^3.0",
        "phpstan/phpstan": "^1.9",
        "phpunit/phpunit": "^10.1"
    },
    "autoload": {
        "psr-4": {
            "Monolog\\": "src/Monolog"
        }
    },
    "autoload-dev": {
        "psr-4": {
            "Monolog\\Test\\": "tests/Monolog"
        }
    }
}"#,
        )
        .unwrap();

        fs::create_dir_all(path.join("src/Monolog")).unwrap();
        fs::write(path.join("composer.lock"), "{}").unwrap();

        let detector = PhpDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Php));
        assert!(detection.confidence > 0.8);
    }

    #[test]
    fn test_no_php_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create a non-PHP file
        fs::write(path.join("package.json"), r#"{"name": "test"}"#).unwrap();
        fs::write(
            path.join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"",
        )
        .unwrap();

        let detector = PhpDetector::new();
        let result = detector.detect(path);

        // Should return an error since no composer.json found
        assert!(result.is_err());
    }

    #[test]
    fn test_detector_name() {
        let detector = PhpDetector::new();
        assert_eq!(detector.name(), "php");
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        let detector = PhpDetector::new();
        let result = detector.detect(path);

        // Should return an error for empty directory with no manifest files
        assert!(result.is_err());
    }

    #[test]
    fn test_composer_json_without_name() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("composer.json"),
            r#"{
    "type": "project",
    "require": {
        "php": ">=8.0"
    }
}"#,
        )
        .unwrap();

        let detector = PhpDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Php));
        // Lower confidence without name, but still above base due to "require"
        assert!(detection.confidence > 0.8);
        assert!(detection.confidence < 0.9);
    }

    #[test]
    fn test_composer_with_vendor_directory_only() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("composer.json"),
            r#"{
    "name": "test/package",
    "require": {
        "monolog/monolog": "^3.0"
    }
}"#,
        )
        .unwrap();

        // Only create vendor directory, no composer.lock
        fs::create_dir_all(path.join("vendor")).unwrap();

        let detector = PhpDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Php));
        assert!(detection.evidence.contains(&"found vendor".to_string()));
    }

    #[test]
    fn test_wordpress_plugin_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("composer.json"),
            r#"{
    "name": "wpackagist-plugin/akismet",
    "version": "5.2",
    "type": "wordpress-plugin",
    "require": {
        "php": ">=5.6"
    },
    "autoload": {
        "files": ["akismet.php"]
    }
}"#,
        )
        .unwrap();

        fs::write(
            path.join("index.php"),
            "<?php\n// WordPress plugin main file",
        )
        .unwrap();

        let detector = PhpDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Php));
        assert!(detection.confidence > 0.7);
    }

    #[test]
    fn test_case_sensitivity_in_composer_json() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Test with different casing (should still work since JSON is case-sensitive)
        fs::write(
            path.join("composer.json"),
            r#"{
    "Name": "test/package",
    "Version": "1.0.0",
    "Require": {
        "php": ">=8.0"
    }
}"#,
        )
        .unwrap();

        let detector = PhpDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Php));
        // Should have base confidence since only composer.json found, no matching patterns
        assert!(detection.confidence >= 0.8);
        assert!(detection.confidence <= 0.8);
    }

    #[test]
    fn test_drupal_module_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("composer.json"),
            r#"{
    "name": "drupal/my_module",
    "type": "drupal-module",
    "require": {
        "php": ">=8.1",
        "drupal/core": "^10.0"
    },
    "autoload": {
        "psr-4": {
            "Drupal\\my_module\\": "src/"
        }
    }
}"#,
        )
        .unwrap();

        fs::create_dir_all(path.join("src")).unwrap();

        let detector = PhpDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Php));
        assert!(detection.confidence > 0.7);
    }

    #[test]
    fn test_detection_with_autoload_php() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("composer.json"),
            r#"{
    "name": "test/package",
    "require": {
        "php": ">=8.0"
    }
}"#,
        )
        .unwrap();

        fs::write(path.join("autoload.php"), "<?php\n// Custom autoload file")
            .unwrap();

        let detector = PhpDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Php));
        assert!(
            detection
                .evidence
                .contains(&"found autoload.php".to_string())
        );
    }
}
