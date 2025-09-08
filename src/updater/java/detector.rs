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

pub struct JavaDetector {}

impl JavaDetector {
    pub fn new() -> Self {
        Self {}
    }
}

impl JavaDetector {
    /// Detect Maven projects using pom.xml
    fn detect_maven(&self, path: &Path) -> Result<FrameworkDetection> {
        let pattern = DetectionPattern {
            manifest_files: vec!["pom.xml"],
            support_files: vec![
                "target",
                "src/main/java",
                "src/test/java",
                "mvnw",
                "mvnw.cmd",
                ".mvn",
            ],
            content_patterns: vec![
                "<groupId>",
                "<artifactId>",
                "<version>",
                "<dependencies>",
                "<packaging>",
            ],
            base_confidence: 0.9,
        };

        DetectionHelper::analyze_with_pattern(
            path,
            pattern.clone(),
            |support_evidence| FrameworkDetection {
                framework: Framework::Java,
                confidence: DetectionHelper::calculate_confidence(
                    &pattern,
                    &support_evidence,
                ),
                evidence: support_evidence,
            },
        )
    }

    /// Detect Gradle projects using build.gradle
    fn detect_gradle(&self, path: &Path) -> Result<FrameworkDetection> {
        let pattern = DetectionPattern {
            manifest_files: vec!["build.gradle", "build.gradle.kts"],
            support_files: vec![
                "build",
                "gradle",
                "gradlew",
                "gradlew.bat",
                "gradle.properties",
                "settings.gradle",
                "settings.gradle.kts",
                "src/main/java",
                "src/test/java",
            ],
            content_patterns: vec![
                "group",
                "version",
                "dependencies",
                "apply plugin",
                "plugins",
            ],
            base_confidence: 0.9,
        };

        DetectionHelper::analyze_with_pattern(
            path,
            pattern.clone(),
            |support_evidence| FrameworkDetection {
                framework: Framework::Java,
                confidence: DetectionHelper::calculate_confidence(
                    &pattern,
                    &support_evidence,
                ),
                evidence: support_evidence,
            },
        )
    }
}

impl FrameworkDetector for JavaDetector {
    fn name(&self) -> &str {
        "java"
    }

    fn detect(&self, path: &Path) -> Result<FrameworkDetection> {
        // Check for Maven first (more common)
        if let Ok(detection) = self.detect_maven(path) {
            return Ok(detection);
        }

        // Fall back to Gradle
        self.detect_gradle(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_maven_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create pom.xml
        fs::write(
            path.join("pom.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0
                             http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>

    <groupId>com.example</groupId>
    <artifactId>test-app</artifactId>
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

        // Create supporting files
        fs::create_dir_all(path.join("src/main/java")).unwrap();
        fs::create_dir_all(path.join("target")).unwrap();

        let detector = JavaDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Java));
        assert!(detection.confidence > 0.8);
        assert!(detection.evidence.contains(&"found pom.xml".to_string()));
        assert!(
            detection
                .evidence
                .contains(&"contains <groupId>".to_string())
        );
        assert!(
            detection
                .evidence
                .contains(&"contains <artifactId>".to_string())
        );
    }

    #[test]
    fn test_gradle_project_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create build.gradle
        fs::write(
            path.join("build.gradle"),
            r#"plugins {
    id 'java'
    id 'application'
}

group = 'com.example'
version = '1.0.0'

repositories {
    mavenCentral()
}

dependencies {
    implementation 'com.google.guava:guava:31.1-jre'
    testImplementation 'junit:junit:4.13.2'
}

application {
    mainClass = 'com.example.App'
}
"#,
        )
        .unwrap();

        // Create supporting files
        fs::create_dir_all(path.join("gradle")).unwrap();
        fs::create_dir_all(path.join("src/main/java")).unwrap();
        fs::write(path.join("gradlew"), "#!/bin/bash\n# Gradle wrapper script")
            .unwrap();

        let detector = JavaDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Java));
        assert!(detection.confidence > 0.8);
        assert!(
            detection
                .evidence
                .contains(&"found build.gradle".to_string())
        );
        assert!(detection.evidence.contains(&"contains plugins".to_string()));
        assert!(detection.evidence.contains(&"contains group".to_string()));
        assert!(detection.evidence.contains(&"contains version".to_string()));
    }

    #[test]
    fn test_gradle_kotlin_dsl_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create build.gradle.kts
        fs::write(
            path.join("build.gradle.kts"),
            r#"plugins {
    java
    application
}

group = "com.example"
version = "1.0.0"

repositories {
    mavenCentral()
}

dependencies {
    implementation("com.google.guava:guava:31.1-jre")
    testImplementation("junit:junit:4.13.2")
}

application {
    mainClass.set("com.example.App")
}
"#,
        )
        .unwrap();

        fs::create_dir_all(path.join("src/main/java")).unwrap();

        let detector = JavaDetector::new();
        let detection = detector.detect_gradle(path).unwrap();

        assert!(matches!(detection.framework, Framework::Java));
        assert!(detection.confidence > 0.8);
        assert!(
            detection
                .evidence
                .contains(&"found build.gradle.kts".to_string())
        );
    }

    #[test]
    fn test_spring_boot_maven_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("pom.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>

    <parent>
        <groupId>org.springframework.boot</groupId>
        <artifactId>spring-boot-starter-parent</artifactId>
        <version>3.1.0</version>
        <relativePath/>
    </parent>

    <groupId>com.example</groupId>
    <artifactId>spring-boot-app</artifactId>
    <version>0.0.1-SNAPSHOT</version>
    <packaging>jar</packaging>

    <dependencies>
        <dependency>
            <groupId>org.springframework.boot</groupId>
            <artifactId>spring-boot-starter-web</artifactId>
        </dependency>
    </dependencies>
</project>"#,
        )
        .unwrap();

        fs::create_dir_all(path.join("src/main/java/com/example")).unwrap();

        let detector = JavaDetector::new();
        let detection = detector.detect_maven(path).unwrap();

        assert!(matches!(detection.framework, Framework::Java));
        assert!(detection.confidence > 0.9);
    }

    #[test]
    fn test_gradle_with_wrapper_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("build.gradle"),
            r#"apply plugin: 'java'

group = 'com.example'
version = '1.0-SNAPSHOT'

dependencies {
    compile 'org.apache.commons:commons-lang3:3.12.0'
}
"#,
        )
        .unwrap();

        // Create Gradle wrapper files
        fs::write(path.join("gradlew"), "#!/bin/bash").unwrap();
        fs::write(path.join("gradlew.bat"), "@echo off").unwrap();
        fs::create_dir_all(path.join("gradle/wrapper")).unwrap();
        fs::write(
            path.join("gradle.properties"),
            "org.gradle.jvmargs=-Xmx2048m",
        )
        .unwrap();

        let detector = JavaDetector::new();
        let detection = detector.detect_gradle(path).unwrap();

        assert!(matches!(detection.framework, Framework::Java));
        assert!(detection.confidence > 0.9);
        assert!(detection.evidence.contains(&"found gradlew".to_string()));
        assert!(
            detection
                .evidence
                .contains(&"found gradle.properties".to_string())
        );
    }

    #[test]
    fn test_maven_with_wrapper_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>maven-wrapper-example</artifactId>
    <version>1.0.0</version>
    <dependencies>
        <dependency>
            <groupId>org.junit.jupiter</groupId>
            <artifactId>junit-jupiter</artifactId>
            <version>5.9.0</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
</project>"#,
        )
        .unwrap();

        // Create Maven wrapper files
        fs::write(path.join("mvnw"), "#!/bin/bash").unwrap();
        fs::write(path.join("mvnw.cmd"), "@echo off").unwrap();
        fs::create_dir_all(path.join(".mvn/wrapper")).unwrap();

        let detector = JavaDetector::new();
        let detection = detector.detect_maven(path).unwrap();

        assert!(matches!(detection.framework, Framework::Java));
        assert!(detection.confidence > 0.9);
        assert!(detection.evidence.contains(&"found mvnw".to_string()));
        assert!(detection.evidence.contains(&"found .mvn".to_string()));
    }

    #[test]
    fn test_multi_module_maven_project() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Root pom.xml
        fs::write(
            path.join("pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>multi-module-parent</artifactId>
    <version>1.0.0</version>
    <packaging>pom</packaging>

    <modules>
        <module>core</module>
        <module>web</module>
    </modules>
</project>"#,
        )
        .unwrap();

        fs::create_dir_all(path.join("core")).unwrap();
        fs::create_dir_all(path.join("web")).unwrap();

        let detector = JavaDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Java));
        assert!(detection.confidence > 0.8);
        assert!(
            detection
                .evidence
                .contains(&"contains <packaging>".to_string())
        );
    }

    #[test]
    fn test_detect_prefers_maven_over_gradle() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create both Maven and Gradle files
        fs::write(
            path.join("pom.xml"),
            r#"<project>
    <groupId>com.example</groupId>
    <artifactId>hybrid-project</artifactId>
    <version>1.0.0</version>
</project>"#,
        )
        .unwrap();

        fs::write(
            path.join("build.gradle"),
            r#"group = 'com.example'
version = '1.0.0'
"#,
        )
        .unwrap();

        let detector = JavaDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Java));
        // Should prefer Maven (higher confidence)
        assert!(detection.confidence > 0.8);
        assert!(detection.evidence.contains(&"found pom.xml".to_string()));
    }

    #[test]
    fn test_no_java_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create non-Java files
        fs::write(path.join("package.json"), r#"{"name": "test"}"#).unwrap();
        fs::write(
            path.join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"",
        )
        .unwrap();

        let detector = JavaDetector::new();
        let result = detector.detect(path);

        // Should return an error since no Java files found
        assert!(result.is_err());
    }

    #[test]
    fn test_detector_name() {
        let detector = JavaDetector::new();
        assert_eq!(detector.name(), "java");
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        let detector = JavaDetector::new();
        let result = detector.detect(path);

        // Should return an error for empty directory with no manifest files
        assert!(result.is_err());
    }

    #[test]
    fn test_minimal_maven_project() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("pom.xml"),
            r#"<project>
    <groupId>com.example</groupId>
    <artifactId>minimal</artifactId>
</project>"#,
        )
        .unwrap();

        let detector = JavaDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Java));
        // High confidence even with minimal Maven project due to strong XML patterns
        assert!(detection.confidence >= 0.9);
    }

    #[test]
    fn test_minimal_gradle_project() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("build.gradle"),
            r#"apply plugin: 'java'
"#,
        )
        .unwrap();

        let detector = JavaDetector::new();
        let detection = detector.detect_gradle(path).unwrap();

        assert!(matches!(detection.framework, Framework::Java));
        // High confidence even with minimal Gradle project due to plugin detection
        assert!(detection.confidence >= 0.9);
    }

    #[test]
    fn test_android_gradle_project() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("build.gradle"),
            r#"apply plugin: 'com.android.application'

android {
    compileSdkVersion 33

    defaultConfig {
        applicationId "com.example.myapp"
        minSdkVersion 21
        targetSdkVersion 33
        versionCode 1
        versionName "1.0"
    }
}

dependencies {
    implementation 'androidx.appcompat:appcompat:1.6.0'
}
"#,
        )
        .unwrap();

        let detector = JavaDetector::new();
        let detection = detector.detect_gradle(path).unwrap();

        assert!(matches!(detection.framework, Framework::Java));
        assert!(detection.confidence > 0.8);
    }

    #[test]
    fn test_comprehensive_detection_with_all_evidence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create comprehensive Java project structure
        fs::write(
            path.join("pom.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>comprehensive-app</artifactId>
    <version>2.1.0</version>
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

        // Create all supporting files and directories
        fs::create_dir_all(path.join("target")).unwrap();
        fs::create_dir_all(path.join("src/main/java")).unwrap();
        fs::create_dir_all(path.join("src/test/java")).unwrap();
        fs::write(path.join("mvnw"), "#!/bin/bash").unwrap();
        fs::create_dir_all(path.join(".mvn")).unwrap();

        let detector = JavaDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Java));
        // Should have very high confidence with all evidence present
        assert!(detection.confidence > 0.95);

        // Check that multiple pieces of evidence were found
        assert!(detection.evidence.contains(&"found pom.xml".to_string()));
        assert!(detection.evidence.contains(&"found target".to_string()));
        assert!(
            detection
                .evidence
                .contains(&"found src/main/java".to_string())
        );
        assert!(
            detection
                .evidence
                .contains(&"found src/test/java".to_string())
        );
        assert!(detection.evidence.contains(&"found mvnw".to_string()));
        assert!(detection.evidence.contains(&"found .mvn".to_string()));
        assert!(
            detection
                .evidence
                .contains(&"contains <groupId>".to_string())
        );
        assert!(
            detection
                .evidence
                .contains(&"contains <artifactId>".to_string())
        );
        assert!(
            detection
                .evidence
                .contains(&"contains <version>".to_string())
        );
        assert!(
            detection
                .evidence
                .contains(&"contains <dependencies>".to_string())
        );
        assert!(
            detection
                .evidence
                .contains(&"contains <packaging>".to_string())
        );
    }
}
