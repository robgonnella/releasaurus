use log::*;
use quick_xml::events::{BytesText, Event};
use quick_xml::{Reader, Writer};
use regex::Regex;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

use crate::{
    result::Result,
    updater::framework::Framework,
    updater::{framework::Package, traits::PackageUpdater},
};

/// Java package updater supporting Maven and Gradle projects.
pub struct JavaUpdater {}

impl JavaUpdater {
    pub fn new() -> Self {
        Self {}
    }

    /// Process packages and update their build files
    fn process_packages(&self, packages: &[Package]) -> Result<()> {
        for package in packages {
            let package_path = Path::new(&package.path);

            // Try Maven first (pom.xml)
            let pom_path = package_path.join("pom.xml");
            if pom_path.exists() {
                info!("Updating Maven project: {}", package.path);
                self.update_maven_project(&pom_path, package)?;
                continue;
            }

            // Try Gradle (build.gradle or build.gradle.kts)
            let gradle_path = package_path.join("build.gradle");
            let gradle_kts_path = package_path.join("build.gradle.kts");

            if gradle_path.exists() {
                info!("Updating Gradle project: {}", package.path);
                self.update_gradle_project(&gradle_path, package, false)?;
            } else if gradle_kts_path.exists() {
                info!("Updating Gradle Kotlin DSL project: {}", package.path);
                self.update_gradle_project(&gradle_kts_path, package, true)?;
            } else {
                info!(
                    "No Maven or Gradle build file found for package: {}",
                    package.path
                );
            }

            // Also check for gradle.properties
            let gradle_props_path = package_path.join("gradle.properties");
            if gradle_props_path.exists() {
                self.update_gradle_properties(&gradle_props_path, package)?;
            }
        }

        Ok(())
    }

    /// Update Maven project by modifying pom.xml
    fn update_maven_project(
        &self,
        pom_path: &Path,
        package: &Package,
    ) -> Result<()> {
        let mut file = File::open(pom_path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;

        let mut reader = Reader::from_reader(content.as_slice());

        let mut writer = Writer::new(Vec::new());
        let mut in_project_version = false;
        let mut in_version_element = false;
        let mut depth = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    depth += 1;
                    if e.name().as_ref() == b"version" {
                        // Check if we're at the project level (depth should be 2: project > version)
                        if depth == 2 {
                            in_project_version = true;
                        }
                        in_version_element = true;
                    }
                    writer.write_event(Event::Start(e.clone()))?;
                }
                Ok(Event::End(ref e)) => {
                    depth -= 1;
                    if e.name().as_ref() == b"version" {
                        in_version_element = false;
                        if in_project_version {
                            in_project_version = false;
                        }
                    }
                    writer.write_event(Event::End(e.clone()))?;
                }
                Ok(Event::Text(ref e)) => {
                    if in_project_version && in_version_element {
                        // Replace the version text
                        let new_version =
                            package.next_version.semver.to_string();
                        info!("Updating Maven version to: {}", new_version);
                        writer.write_event(Event::Text(BytesText::new(
                            &new_version,
                        )))?;
                    } else {
                        writer.write_event(Event::Text(e.clone()))?;
                    }
                }
                Ok(Event::Eof) => break,
                Ok(e) => writer.write_event(e)?,
                Err(e) => return Err(e.into()),
            }
        }

        let result = writer.into_inner();
        let mut output_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(pom_path)?;
        output_file.write_all(&result)?;

        Ok(())
    }

    /// Update Gradle project by modifying build.gradle or build.gradle.kts
    fn update_gradle_project(
        &self,
        build_path: &Path,
        package: &Package,
        is_kotlin: bool,
    ) -> Result<()> {
        let mut file = File::open(build_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let new_version = package.next_version.semver.to_string();

        // Define regex patterns for different version declaration styles
        let patterns = if is_kotlin {
            vec![
                // Kotlin DSL patterns
                Regex::new(r#"version\s*=\s*"[^"]*""#)?,
                Regex::new(r#"version\s*=\s*'[^']*'"#)?,
                Regex::new(r#"val\s+version\s*=\s*"[^"]*""#)?,
                Regex::new(r#"val\s+version\s*=\s*'[^']*'"#)?,
                Regex::new(r#"project\.version\s*=\s*"[^"]*""#)?,
                Regex::new(r#"project\.version\s*=\s*'[^']*'"#)?,
            ]
        } else {
            vec![
                // Groovy DSL patterns
                Regex::new(r#"version\s*=\s*["'][^"']*["']"#)?,
                Regex::new(r#"version\s+["'][^"']*["']"#)?,
                Regex::new(r#"def\s+version\s*=\s*["'][^"']*["']"#)?,
                Regex::new(r#"project\.version\s*=\s*["'][^"']*["']"#)?,
            ]
        };

        let mut updated_content = content.clone();
        let mut version_found = false;

        for pattern in patterns {
            if pattern.is_match(&content) {
                if is_kotlin {
                    updated_content = pattern
                        .replace_all(
                            &updated_content,
                            |caps: &regex::Captures| {
                                let full_match = caps.get(0).unwrap().as_str();
                                if full_match.contains('"') {
                                    format!(
                                        "{}\"{}\"",
                                        full_match
                                            .split('"')
                                            .next()
                                            .unwrap_or(""),
                                        new_version
                                    )
                                } else {
                                    format!(
                                        "{}'{}\'",
                                        full_match
                                            .split('\'')
                                            .next()
                                            .unwrap_or(""),
                                        new_version
                                    )
                                }
                            },
                        )
                        .to_string();
                } else {
                    updated_content = pattern
                        .replace_all(
                            &updated_content,
                            |caps: &regex::Captures| {
                                let full_match = caps.get(0).unwrap().as_str();
                                let quote_char = if full_match.contains('"') {
                                    '"'
                                } else {
                                    '\''
                                };
                                let prefix = full_match
                                    .split(quote_char)
                                    .next()
                                    .unwrap_or("");
                                format!(
                                    "{}{}{}{}",
                                    prefix, quote_char, new_version, quote_char
                                )
                            },
                        )
                        .to_string();
                }
                version_found = true;
                break;
            }
        }

        if version_found {
            info!("Updating Gradle version to: {}", new_version);
            let mut output_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(build_path)?;
            output_file.write_all(updated_content.as_bytes())?;
        } else {
            info!(
                "No version declaration found in Gradle build file: {}",
                build_path.display()
            );
        }

        Ok(())
    }

    /// Update gradle.properties file
    fn update_gradle_properties(
        &self,
        props_path: &Path,
        package: &Package,
    ) -> Result<()> {
        let file = File::open(props_path)?;
        let reader = BufReader::new(file);
        let mut lines: Vec<String> = Vec::new();
        let mut version_updated = false;

        let new_version = package.next_version.semver.to_string();

        // Read all lines and update version property
        for line in reader.lines() {
            let line = line?;
            if line.trim_start().starts_with("version") && line.contains('=') {
                lines.push(format!("version={}", new_version));
                version_updated = true;
                info!(
                    "Updated version in gradle.properties to: {}",
                    new_version
                );
            } else {
                lines.push(line);
            }
        }

        // Only write back if we actually updated something
        if version_updated {
            let mut output_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(props_path)?;

            for line in lines {
                writeln!(output_file, "{}", line)?;
            }
        }

        Ok(())
    }
}

impl PackageUpdater for JavaUpdater {
    fn update(&self, root_path: &Path, packages: Vec<Package>) -> Result<()> {
        let java_packages = packages
            .into_iter()
            .filter(|p| matches!(p.framework, Framework::Java))
            .collect::<Vec<Package>>();

        info!(
            "Found {} Java packages in {}",
            java_packages.len(),
            root_path.display(),
        );

        self.process_packages(&java_packages)?;

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
    fn test_java_updater_creation() {
        let _updater = JavaUpdater::new();
        // Basic test to ensure the updater can be created without panicking
    }

    #[test]
    fn test_java_updater_empty_packages() {
        let updater = JavaUpdater::new();
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let packages = vec![];

        let result = updater.update(path, packages);
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_filters_java_packages_only() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create Java package
        let java_dir = root_path.join("java-package");
        fs::create_dir_all(&java_dir).unwrap();
        fs::write(
            java_dir.join("pom.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>test-package</artifactId>
    <version>1.0.0</version>
</project>"#,
        )
        .unwrap();

        // Create non-Java package
        let node_dir = root_path.join("node-package");
        fs::create_dir_all(&node_dir).unwrap();

        let packages = vec![
            create_test_package(
                "com.example:test-package",
                java_dir.to_str().unwrap(),
                "2.0.0",
                Framework::Java,
            ),
            create_test_package(
                "node-package",
                node_dir.to_str().unwrap(),
                "2.0.0",
                Framework::Node,
            ),
        ];

        let updater = JavaUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Check that Java package was updated
        let updated_content =
            fs::read_to_string(java_dir.join("pom.xml")).unwrap();
        assert!(updated_content.contains("<version>2.0.0</version>"));
    }

    #[test]
    fn test_update_maven_project() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("maven-project");
        fs::create_dir_all(&package_dir).unwrap();

        // Create initial pom.xml
        fs::write(
            package_dir.join("pom.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0
         http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>maven-project</artifactId>
    <version>1.0.0</version>
    <packaging>jar</packaging>

    <properties>
        <maven.compiler.source>11</maven.compiler.source>
        <maven.compiler.target>11</maven.compiler.target>
    </properties>
</project>"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "com.example:maven-project",
            package_dir.to_str().unwrap(),
            "2.1.0",
            Framework::Java,
        )];

        let updater = JavaUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Verify the version was updated
        let updated_content =
            fs::read_to_string(package_dir.join("pom.xml")).unwrap();
        assert!(updated_content.contains("<version>2.1.0</version>"));

        // Verify other elements remain unchanged
        assert!(updated_content.contains("<groupId>com.example</groupId>"));
        assert!(
            updated_content.contains("<artifactId>maven-project</artifactId>")
        );
        assert!(updated_content.contains("<packaging>jar</packaging>"));
    }

    #[test]
    fn test_update_gradle_project() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("gradle-project");
        fs::create_dir_all(&package_dir).unwrap();

        // Create initial build.gradle
        fs::write(
            package_dir.join("build.gradle"),
            r#"plugins {
    id 'java'
}

group = 'com.example'
version = '1.0.0'
sourceCompatibility = '11'

repositories {
    mavenCentral()
}

dependencies {
    testImplementation 'junit:junit:4.13.2'
}"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "com.example:gradle-project",
            package_dir.to_str().unwrap(),
            "2.1.0",
            Framework::Java,
        )];

        let updater = JavaUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Verify the version was updated
        let updated_content =
            fs::read_to_string(package_dir.join("build.gradle")).unwrap();
        assert!(updated_content.contains("version = '2.1.0'"));
        assert!(updated_content.contains("group = 'com.example'"));
    }

    #[test]
    fn test_update_gradle_kotlin_dsl_project() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("gradle-kotlin-project");
        fs::create_dir_all(&package_dir).unwrap();

        // Create initial build.gradle.kts
        fs::write(
            package_dir.join("build.gradle.kts"),
            r#"plugins {
    java
}

group = "com.example"
version = "1.0.0"

java.sourceCompatibility = JavaVersion.VERSION_11

repositories {
    mavenCentral()
}

dependencies {
    testImplementation("junit:junit:4.13.2")
}"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "com.example:gradle-kotlin-project",
            package_dir.to_str().unwrap(),
            "2.1.0",
            Framework::Java,
        )];

        let updater = JavaUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Verify the version was updated
        let updated_content =
            fs::read_to_string(package_dir.join("build.gradle.kts")).unwrap();
        assert!(updated_content.contains("version = \"2.1.0\""));
        assert!(updated_content.contains("group = \"com.example\""));
    }

    #[test]
    fn test_update_gradle_properties() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("gradle-props-project");
        fs::create_dir_all(&package_dir).unwrap();

        // Create gradle.properties
        fs::write(
            package_dir.join("gradle.properties"),
            r#"# Project properties
group=com.example
version=1.0.0
sourceCompatibility=11

# Other properties
org.gradle.jvmargs=-Xmx2048m
"#,
        )
        .unwrap();

        // Also create a minimal build.gradle so it's recognized as a Java project
        fs::write(
            package_dir.join("build.gradle"),
            r#"plugins {
    id 'java'
}"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "com.example:gradle-props-project",
            package_dir.to_str().unwrap(),
            "2.1.0",
            Framework::Java,
        )];

        let updater = JavaUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Verify the version was updated in gradle.properties
        let updated_content =
            fs::read_to_string(package_dir.join("gradle.properties")).unwrap();
        assert!(updated_content.contains("version=2.1.0"));
        assert!(updated_content.contains("group=com.example"));
        assert!(updated_content.contains("sourceCompatibility=11"));
    }

    #[test]
    fn test_update_with_missing_build_files() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create package directory without build files
        let package_dir = root_path.join("no-build-files");
        fs::create_dir_all(&package_dir).unwrap();

        let packages = vec![create_test_package(
            "no-build-files",
            package_dir.to_str().unwrap(),
            "1.0.0",
            Framework::Java,
        )];

        let updater = JavaUpdater::new();
        let result = updater.update(root_path, packages);
        // Should succeed but skip the package
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_multiple_java_packages() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create Maven package
        let maven_dir = root_path.join("maven-package");
        fs::create_dir_all(&maven_dir).unwrap();
        fs::write(
            maven_dir.join("pom.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>maven-package</artifactId>
    <version>1.0.0</version>
</project>"#,
        )
        .unwrap();

        // Create Gradle package
        let gradle_dir = root_path.join("gradle-package");
        fs::create_dir_all(&gradle_dir).unwrap();
        fs::write(
            gradle_dir.join("build.gradle"),
            r#"plugins {
    id 'java'
}

group = 'com.example'
version = '0.5.0'"#,
        )
        .unwrap();

        let packages = vec![
            create_test_package(
                "com.example:maven-package",
                maven_dir.to_str().unwrap(),
                "1.1.0",
                Framework::Java,
            ),
            create_test_package(
                "com.example:gradle-package",
                gradle_dir.to_str().unwrap(),
                "0.6.0",
                Framework::Java,
            ),
        ];

        let updater = JavaUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        // Verify both packages were updated
        let maven_content =
            fs::read_to_string(maven_dir.join("pom.xml")).unwrap();
        assert!(maven_content.contains("<version>1.1.0</version>"));

        let gradle_content =
            fs::read_to_string(gradle_dir.join("build.gradle")).unwrap();
        assert!(gradle_content.contains("version = '0.6.0'"));
    }

    #[test]
    fn test_gradle_version_variations() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Test different Gradle version declaration styles
        let test_cases = vec![
            ("version = '1.0.0'", "version = '2.0.0'"),
            ("version = \"1.0.0\"", "version = \"2.0.0\""),
            ("version '1.0.0'", "version '2.0.0'"),
            ("def version = '1.0.0'", "def version = '2.0.0'"),
            ("project.version = '1.0.0'", "project.version = '2.0.0'"),
        ];

        for (i, (original, expected)) in test_cases.into_iter().enumerate() {
            let package_dir = root_path.join(format!("gradle-test-{}", i));
            fs::create_dir_all(&package_dir).unwrap();

            let build_content = format!(
                r#"plugins {{
    id 'java'
}}

group = 'com.example'
{}
"#,
                original
            );

            fs::write(package_dir.join("build.gradle"), build_content).unwrap();

            let packages = vec![create_test_package(
                &format!("com.example:gradle-test-{}", i),
                package_dir.to_str().unwrap(),
                "2.0.0",
                Framework::Java,
            )];

            let updater = JavaUpdater::new();
            let result = updater.update(root_path, packages);
            assert!(result.is_ok());

            let updated_content =
                fs::read_to_string(package_dir.join("build.gradle")).unwrap();
            assert!(
                updated_content.contains(expected),
                "Expected '{}' but got content: {}",
                expected,
                updated_content
            );
        }
    }

    #[test]
    fn test_maven_preserves_xml_structure() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        let package_dir = root_path.join("complex-maven");
        fs::create_dir_all(&package_dir).unwrap();

        // Create complex pom.xml
        fs::write(
            package_dir.join("pom.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0
         http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>

    <parent>
        <groupId>org.springframework.boot</groupId>
        <artifactId>spring-boot-starter-parent</artifactId>
        <version>2.7.0</version>
        <relativePath/>
    </parent>

    <groupId>com.example</groupId>
    <artifactId>complex-maven</artifactId>
    <version>1.0.0</version>
    <packaging>jar</packaging>

    <name>Complex Maven Project</name>
    <description>A complex maven project for testing</description>

    <properties>
        <java.version>11</java.version>
        <junit.version>5.8.2</junit.version>
    </properties>

    <dependencies>
        <dependency>
            <groupId>org.springframework.boot</groupId>
            <artifactId>spring-boot-starter-web</artifactId>
        </dependency>
        <dependency>
            <groupId>org.junit.jupiter</groupId>
            <artifactId>junit-jupiter</artifactId>
            <version>${junit.version}</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
</project>"#,
        )
        .unwrap();

        let packages = vec![create_test_package(
            "com.example:complex-maven",
            package_dir.to_str().unwrap(),
            "1.2.3",
            Framework::Java,
        )];

        let updater = JavaUpdater::new();
        let result = updater.update(root_path, packages);
        assert!(result.is_ok());

        let updated_content =
            fs::read_to_string(package_dir.join("pom.xml")).unwrap();

        // Verify version was updated
        assert!(updated_content.contains("<version>1.2.3</version>"));

        // Verify other structure is preserved
        assert!(updated_content.contains("<groupId>com.example</groupId>"));
        assert!(
            updated_content.contains("<artifactId>complex-maven</artifactId>")
        );
        assert!(updated_content.contains("<name>Complex Maven Project</name>"));
        assert!(updated_content.contains("<parent>"));
        assert!(updated_content.contains("spring-boot-starter-parent"));
        assert!(updated_content.contains("<properties>"));
        assert!(updated_content.contains("<dependencies>"));
    }
}
