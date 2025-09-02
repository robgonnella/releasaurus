use crate::updater::{
    detection::{
        helper::DetectionHelper,
        traits::FrameworkDetector,
        types::{DetectionPattern, FrameworkDetection},
    },
    framework::{Framework, Language},
    python::types::PythonMetadata,
};
use color_eyre::eyre::Result;
use std::path::Path;

pub struct PythonDetector {}

impl PythonDetector {
    pub fn new() -> Self {
        Self {}
    }
}

impl PythonDetector {
    /// Detect Python projects using pyproject.toml
    fn detect_python_pyproject(
        &self,
        path: &Path,
    ) -> Result<FrameworkDetection> {
        let pattern = DetectionPattern {
            manifest_files: vec!["pyproject.toml"],
            support_files: vec![
                "poetry.lock",
                "Pipfile",
                "requirements.txt",
                "setup.py",
            ],
            content_patterns: vec![
                "[build-system]",
                "[tool.poetry]",
                "[tool.setuptools]",
            ],
            base_confidence: 0.9,
        };

        DetectionHelper::analyze_with_pattern(
            path,
            pattern.clone(),
            |manifest_content, support_evidence| {
                // Detect build system
                let build_system = if manifest_content.contains("[tool.poetry]")
                {
                    "poetry".to_string()
                } else if manifest_content.contains("[tool.setuptools]") {
                    "setuptools".to_string()
                } else if manifest_content.contains("[tool.flit]") {
                    "flit".to_string()
                } else {
                    "setuptools".to_string()
                };

                // Detect package manager
                let package_manager = if path.join("poetry.lock").exists() {
                    "poetry".to_string()
                } else if path.join("Pipfile").exists() {
                    "pipenv".to_string()
                } else {
                    "pip".to_string()
                };

                let metadata = PythonMetadata {
                    build_system,
                    package_manager,
                    uses_pyproject: true,
                };

                FrameworkDetection {
                    framework: Framework::Python(Language {
                        name: self.name().into(),
                        manifest_path: path.join("pyproject.toml"),
                        metadata,
                    }),
                    confidence: DetectionHelper::calculate_confidence(
                        &pattern,
                        &support_evidence,
                    ),
                    evidence: support_evidence,
                }
            },
        )
    }

    /// Detect Python projects using setup.py/setup.cfg
    fn detect_python_setuptools(
        &self,
        path: &Path,
    ) -> Result<FrameworkDetection> {
        let pattern = DetectionPattern {
            manifest_files: vec!["setup.py"],
            support_files: vec!["requirements.txt", "MANIFEST.in", "tox.ini"],
            content_patterns: vec!["from setuptools", "setup(", "[metadata]"],
            base_confidence: 0.7,
        };

        DetectionHelper::analyze_with_pattern(
            path,
            pattern.clone(),
            |_manifest_content, support_evidence| {
                let metadata = PythonMetadata {
                    build_system: "setuptools".to_string(),
                    package_manager: "pip".to_string(),
                    uses_pyproject: false,
                };

                FrameworkDetection {
                    framework: Framework::Python(Language {
                        name: self.name().into(),
                        manifest_path: path.join("setup.py"),
                        metadata,
                    }),
                    confidence: DetectionHelper::calculate_confidence(
                        &pattern,
                        &support_evidence,
                    ),
                    evidence: support_evidence,
                }
            },
        )
    }
}

impl FrameworkDetector for PythonDetector {
    fn name(&self) -> &str {
        "python"
    }

    fn detect(&self, path: &Path) -> Result<FrameworkDetection> {
        // Check for pyproject.toml first (modern Python)
        if let Ok(detection) = self.detect_python_pyproject(path) {
            return Ok(detection);
        }

        // Fall back to setup.py/setup.cfg
        self.detect_python_setuptools(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_python_pyproject_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create pyproject.toml
        fs::write(
            path.join("pyproject.toml"),
            r#"[build-system]
requires = ["poetry-core>=1.0.0"]
build-backend = "poetry.core.masonry.api"

[tool.poetry]
name = "test-lib"
version = "0.1.0"
description = ""
authors = ["Test Author <test@example.com>"]

[tool.poetry.dependencies]
python = "^3.8"
"#,
        )
        .unwrap();

        // Create supporting files
        fs::write(path.join("poetry.lock"), "").unwrap();

        let detector = PythonDetector::new();
        let detection = detector.detect(path).unwrap();

        match detection.framework {
            Framework::Python(lang) => {
                assert!(lang.metadata.uses_pyproject);
                assert_eq!(lang.metadata.build_system, "poetry");
                assert_eq!(lang.metadata.package_manager, "poetry");
            }
            _ => panic!("Expected Python framework"),
        }

        assert!(detection.confidence > 0.8);
        assert!(
            detection
                .evidence
                .contains(&"found pyproject.toml".to_string())
        );
    }
}
