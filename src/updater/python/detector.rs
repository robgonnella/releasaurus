use color_eyre::eyre::Result;
use std::path::Path;

use crate::updater::{
    detection::{
        helper::DetectionHelper,
        traits::FrameworkDetector,
        types::{DetectionPattern, FrameworkDetection},
    },
    framework::Framework,
};

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
                "setup.cfg",
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
            |support_evidence| FrameworkDetection {
                framework: Framework::Python,
                confidence: DetectionHelper::calculate_confidence(
                    &pattern,
                    &support_evidence,
                ),
                evidence: support_evidence,
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
            support_files: vec![
                "setup.cfg",
                "requirements.txt",
                "MANIFEST.in",
                "tox.ini",
            ],
            content_patterns: vec!["from setuptools", "setup(", "[metadata]"],
            base_confidence: 0.7,
        };

        DetectionHelper::analyze_with_pattern(
            path,
            pattern.clone(),
            |support_evidence| FrameworkDetection {
                framework: Framework::Python,
                confidence: DetectionHelper::calculate_confidence(
                    &pattern,
                    &support_evidence,
                ),
                evidence: support_evidence,
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

        assert!(matches!(detection.framework, Framework::Python));

        assert!(detection.confidence > 0.8);

        assert!(
            detection
                .evidence
                .contains(&"found pyproject.toml".to_string())
        );
    }

    #[test]
    fn test_python_setuptools_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create setup.py
        fs::write(
            path.join("setup.py"),
            r#"from setuptools import setup, find_packages

setup(
    name="test-package",
    version="0.1.0",
    packages=find_packages(),
    install_requires=[
        "requests>=2.20.0",
        "click>=7.0",
    ],
    python_requires=">=3.8",
)
"#,
        )
        .unwrap();

        // Create supporting files
        fs::write(path.join("setup.cfg"), "[metadata]\nname = test-package")
            .unwrap();
        fs::write(
            path.join("requirements.txt"),
            "requests>=2.20.0\nclick>=7.0",
        )
        .unwrap();

        let detector = PythonDetector::new();
        let detection = detector.detect_python_setuptools(path).unwrap();

        assert!(matches!(detection.framework, Framework::Python));

        assert!(detection.confidence > 0.6);

        assert!(detection.evidence.contains(&"found setup.py".to_string()));
    }
}
