use std::path::PathBuf;

use crate::analyzer::types::Version;
use crate::updater::detection::manager::DetectionManager;
use crate::updater::detection::traits::FrameworkDetector;
use crate::updater::generic::updater::GenericUpdater;
use crate::updater::java::detector::JavaDetector;
use crate::updater::java::updater::JavaUpdater;
use crate::updater::node::detector::NodeDetector;
use crate::updater::node::updater::NodeUpdater;
use crate::updater::php::detector::PhpDetector;
use crate::updater::php::updater::PhpUpdater;
use crate::updater::python::detector::PythonDetector;
use crate::updater::python::updater::PythonUpdater;
use crate::updater::rust::detector::RustDetector;
use crate::updater::rust::updater::CargoUpdater;
use crate::updater::traits::PackageUpdater;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
/// Supported frameworks and languages
pub enum Framework {
    /// Rust with Cargo
    Rust,
    /// Node.js with npm/yarn/pnpm
    Node,
    /// Python with pip/setuptools/poetry
    Python,
    /// PHP with Composer
    Php,
    /// Java with Maven/Gradle
    Java,
    #[default]
    /// Generic framework with custom handling
    Generic,
}

impl Framework {
    pub fn detection_manager(root_path: PathBuf) -> DetectionManager {
        let detectors: Vec<Box<dyn FrameworkDetector>> = vec![
            Box::new(RustDetector::new()),
            Box::new(PythonDetector::new()),
            Box::new(NodeDetector::new()),
            Box::new(PhpDetector::new()),
            Box::new(JavaDetector::new()),
        ];

        DetectionManager::new(root_path, detectors)
    }

    pub fn name(&self) -> &str {
        match self {
            Framework::Rust => "rust",
            Framework::Node => "node",
            Framework::Python => "python",
            Framework::Php => "php",
            Framework::Java => "java",
            Framework::Generic => "unknown",
        }
    }

    pub fn updater(&self) -> Box<dyn PackageUpdater> {
        match self {
            Framework::Rust => Box::new(CargoUpdater::new()),
            Framework::Node => Box::new(NodeUpdater::new()),
            Framework::Python => Box::new(PythonUpdater::new()),
            Framework::Php => Box::new(PhpUpdater::new()),
            Framework::Java => Box::new(JavaUpdater::new()),
            Framework::Generic => Box::new(GenericUpdater::new()),
        }
    }
}

/// A language/framework-agnostic package that needs version updates
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// Package name as defined in the manifest file
    pub name: String,
    /// Path to the package directory (relative to repository root)
    pub path: String,
    /// Next version to update to
    pub next_version: Version,
    /// Detected framework/language for this package
    pub framework: Framework,
}

impl Package {
    /// Create a new package with minimal information
    pub fn new(
        name: String,
        path: String,
        next_version: Version,
        framework: Framework,
    ) -> Self {
        Self {
            name,
            path,
            next_version,
            framework: framework.clone(),
        }
    }

    /// Get the framework type as a string
    pub fn framework_name(&self) -> &str {
        self.framework.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_php_detection_integration() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create a basic PHP project with composer.json
        fs::write(
            path.join("composer.json"),
            r#"{
    "name": "test/php-project",
    "version": "1.0.0",
    "require": {
        "php": ">=8.0"
    },
    "autoload": {
        "psr-4": {
            "Test\\": "src/"
        }
    }
}"#,
        )
        .unwrap();

        fs::create_dir_all(path.join("src")).unwrap();

        // Use the detection manager to detect the framework
        let detection_manager =
            Framework::detection_manager(path.to_path_buf());
        let detection_result = detection_manager.detect_framework(".").unwrap();

        // Verify PHP is detected
        assert!(matches!(detection_result.framework, Framework::Php));
        assert!(detection_result.confidence > 0.8);
        assert!(
            detection_result
                .evidence
                .contains(&"found composer.json".to_string())
        );
    }

    #[test]
    fn test_framework_names() {
        assert_eq!(Framework::Rust.name(), "rust");
        assert_eq!(Framework::Node.name(), "node");
        assert_eq!(Framework::Python.name(), "python");
        assert_eq!(Framework::Php.name(), "php");
        assert_eq!(Framework::Java.name(), "java");
        assert_eq!(Framework::Generic.name(), "unknown");
    }

    #[test]
    fn test_java_detection_integration() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create a basic Java project with pom.xml
        fs::write(
            path.join("pom.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>java-test-project</artifactId>
    <version>1.0.0</version>
    <packaging>jar</packaging>

    <dependencies>
        <dependency>
            <groupId>junit</groupId>
            <artifactId>junit</artifactId>
            <version>4.13.2</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
</project>"#,
        )
        .unwrap();

        fs::create_dir_all(path.join("src/main/java")).unwrap();

        // Use the detection manager to detect the framework
        let detection_manager =
            Framework::detection_manager(path.to_path_buf());
        let detection_result = detection_manager.detect_framework(".").unwrap();

        // Verify Java is detected
        assert!(matches!(detection_result.framework, Framework::Java));
        assert!(detection_result.confidence > 0.8);
        assert!(
            detection_result
                .evidence
                .contains(&"found pom.xml".to_string())
        );
    }

    #[test]
    fn test_java_updater_integration() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("java-lib");
        fs::create_dir_all(&package_dir).unwrap();

        // Create initial pom.xml
        fs::write(
            package_dir.join("pom.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>test-lib</artifactId>
    <version>1.5.0</version>
    <packaging>jar</packaging>

    <dependencies>
        <dependency>
            <groupId>org.springframework</groupId>
            <artifactId>spring-core</artifactId>
            <version>6.0.0</version>
        </dependency>
    </dependencies>
</project>"#,
        )
        .unwrap();

        // Create package with Java framework
        let package = Package::new(
            "com.example:test-lib".to_string(),
            package_dir.to_str().unwrap().to_string(),
            Version {
                tag: "v2.0.0".to_string(),
                semver: semver::Version::parse("2.0.0").unwrap(),
            },
            Framework::Java,
        );

        // Get the Java updater from framework and update
        let updater = Framework::Java.updater();
        let result = updater.update(root_path, vec![package]);
        assert!(result.is_ok());

        // Note: The Java updater currently doesn't modify files (placeholder implementation)
        // When fully implemented, this would verify version updates in pom.xml
    }

    #[test]
    fn test_php_updater_integration() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("php-lib");
        fs::create_dir_all(&package_dir).unwrap();

        // Create initial composer.json
        fs::write(
            package_dir.join("composer.json"),
            r#"{
    "name": "vendor/test-lib",
    "version": "1.5.0",
    "type": "library",
    "require": {
        "php": ">=8.0"
    },
    "autoload": {
        "psr-4": {
            "Vendor\\TestLib\\": "src/"
        }
    }
}"#,
        )
        .unwrap();

        // Create package with PHP framework
        let package = Package::new(
            "vendor/test-lib".to_string(),
            package_dir.to_str().unwrap().to_string(),
            Version {
                tag: "v2.0.0".to_string(),
                semver: semver::Version::parse("2.0.0").unwrap(),
            },
            Framework::Php,
        );

        // Get the PHP updater from framework and update
        let updater = Framework::Php.updater();
        let result = updater.update(root_path, vec![package]);
        assert!(result.is_ok());

        // Verify the version was updated
        let updated_content =
            fs::read_to_string(package_dir.join("composer.json")).unwrap();
        assert!(updated_content.contains("\"version\": \"2.0.0\""));
        assert!(updated_content.contains("\"name\": \"vendor/test-lib\""));
        assert!(updated_content.contains("\"php\": \">=8.0\""));
    }

    #[test]
    fn test_json_formatting_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create Node package
        let node_dir = root_path.join("node-package");
        fs::create_dir_all(&node_dir).unwrap();
        fs::write(
            node_dir.join("package.json"),
            r#"{"name":"test-node","version":"1.0.0","dependencies":{"express":"^4.0.0"}}"#,
        )
        .unwrap();

        // Create PHP package
        let php_dir = root_path.join("php-package");
        fs::create_dir_all(&php_dir).unwrap();
        fs::write(
            php_dir.join("composer.json"),
            r#"{"name":"test/php","version":"1.0.0","require":{"php":">=8.0"}}"#,
        )
        .unwrap();

        let packages = vec![
            Package::new(
                "test-node".to_string(),
                node_dir.to_str().unwrap().to_string(),
                Version {
                    tag: "v2.0.0".to_string(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                },
                Framework::Node,
            ),
            Package::new(
                "test/php".to_string(),
                php_dir.to_str().unwrap().to_string(),
                Version {
                    tag: "v2.0.0".to_string(),
                    semver: semver::Version::parse("2.0.0").unwrap(),
                },
                Framework::Php,
            ),
        ];

        // Update both packages
        let node_updater = Framework::Node.updater();
        let php_updater = Framework::Php.updater();

        node_updater
            .update(root_path, vec![packages[0].clone()])
            .unwrap();
        php_updater
            .update(root_path, vec![packages[1].clone()])
            .unwrap();

        // Verify both files use pretty formatting (have spaces after colons)
        let node_content =
            fs::read_to_string(node_dir.join("package.json")).unwrap();
        let php_content =
            fs::read_to_string(php_dir.join("composer.json")).unwrap();

        // Both should have pretty formatting with spaces
        assert!(node_content.contains("\"version\": \"2.0.0\""));
        assert!(node_content.contains("\"name\": \"test-node\""));
        assert!(php_content.contains("\"version\": \"2.0.0\""));
        assert!(php_content.contains("\"name\": \"test/php\""));

        // Both should have proper indentation (newlines and spaces)
        assert!(node_content.contains("\n"));
        assert!(php_content.contains("\n"));
    }
}
