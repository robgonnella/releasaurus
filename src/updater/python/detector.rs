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

    #[test]
    fn test_detector_name() {
        let detector = PythonDetector::new();
        assert_eq!(detector.name(), "python");
    }

    #[test]
    fn test_detect_prefers_pyproject_over_setuptools() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create both pyproject.toml and setup.py
        fs::write(
            path.join("pyproject.toml"),
            r#"[tool.poetry]
name = "test-package"
version = "0.1.0"
"#,
        )
        .unwrap();

        fs::write(
            path.join("setup.py"),
            r#"from setuptools import setup
setup(name="test-package", version="0.1.0")
"#,
        )
        .unwrap();

        let detector = PythonDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Python));
        // Should prefer pyproject.toml (higher confidence)
        assert!(detection.confidence > 0.8);
        assert!(
            detection
                .evidence
                .contains(&"found pyproject.toml".to_string())
        );
    }

    #[test]
    fn test_pyproject_with_setuptools_build_backend() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("pyproject.toml"),
            r#"[build-system]
requires = ["setuptools>=45", "wheel"]
build-backend = "setuptools.build_meta"

[tool.setuptools]
packages = ["my_package"]

[project]
name = "test-package"
version = "0.1.0"
"#,
        )
        .unwrap();

        let detector = PythonDetector::new();
        let detection = detector.detect_python_pyproject(path).unwrap();

        assert!(matches!(detection.framework, Framework::Python));
        assert!(detection.confidence > 0.8);
        assert!(
            detection
                .evidence
                .contains(&"found pyproject.toml".to_string())
        );
    }

    #[test]
    fn test_pyproject_with_flit_backend() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("pyproject.toml"),
            r#"[build-system]
requires = ["flit_core >=3.2,<4"]
build-backend = "flit_core.buildapi"

[project]
name = "my-package"
authors = [{name = "Test Author", email = "test@example.com"}]
dynamic = ["version", "description"]
"#,
        )
        .unwrap();

        let detector = PythonDetector::new();
        let detection = detector.detect_python_pyproject(path).unwrap();

        assert!(matches!(detection.framework, Framework::Python));
        assert!(detection.confidence > 0.8);
    }

    #[test]
    fn test_minimal_pyproject_toml() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("pyproject.toml"),
            r#"[build-system]
requires = ["setuptools"]
"#,
        )
        .unwrap();

        let detector = PythonDetector::new();
        let detection = detector.detect_python_pyproject(path).unwrap();

        assert!(matches!(detection.framework, Framework::Python));
        // Lower confidence without project section
        assert!(detection.confidence > 0.5);
    }

    #[test]
    fn test_setup_py_without_setuptools_import() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("setup.py"),
            r#"from distutils.core import setup

setup(
    name="old-style-package",
    version="0.1.0",
)
"#,
        )
        .unwrap();

        let detector = PythonDetector::new();
        let detection = detector.detect_python_setuptools(path).unwrap();

        assert!(matches!(detection.framework, Framework::Python));
        // Should still detect but with lower confidence
        assert!(detection.confidence > 0.5);
    }

    #[test]
    fn test_detection_with_multiple_support_files() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("pyproject.toml"),
            r#"[tool.poetry]
name = "full-package"
version = "0.1.0"
"#,
        )
        .unwrap();

        // Create many supporting files
        fs::write(path.join("poetry.lock"), "").unwrap();
        fs::write(path.join("requirements.txt"), "requests>=2.0").unwrap();
        fs::write(path.join("setup.py"), "# fallback").unwrap();
        fs::write(path.join("setup.cfg"), "[metadata]\nname=full-package")
            .unwrap();

        let detector = PythonDetector::new();
        let detection = detector.detect_python_pyproject(path).unwrap();

        assert!(matches!(detection.framework, Framework::Python));
        // Should have very high confidence with many supporting files
        assert!(detection.confidence > 0.9);
        assert!(detection.evidence.len() > 3); // Multiple pieces of evidence
    }

    #[test]
    fn test_no_python_files_detected() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create non-Python files
        fs::write(path.join("package.json"), r#"{"name": "node-package"}"#)
            .unwrap();
        fs::write(
            path.join("Cargo.toml"),
            "[package]\nname = \"rust-package\"",
        )
        .unwrap();

        let detector = PythonDetector::new();
        let result = detector.detect(path);

        // Should return an error since no Python files are found
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        let detector = PythonDetector::new();
        let result = detector.detect(path);

        // Should return an error for empty directory
        assert!(result.is_err());
    }

    #[test]
    fn test_pyproject_toml_without_build_system() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("pyproject.toml"),
            r#"[tool.poetry]
name = "no-build-system"
version = "0.1.0"

[tool.black]
line-length = 88
"#,
        )
        .unwrap();

        let detector = PythonDetector::new();
        let detection = detector.detect_python_pyproject(path).unwrap();

        assert!(matches!(detection.framework, Framework::Python));
        assert!(detection.confidence > 0.7);
    }

    #[test]
    fn test_setup_cfg_only() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("setup.cfg"),
            r#"[metadata]
name = cfg-only-package
version = 0.1.0

[options]
packages = find:
install_requires =
    requests
"#,
        )
        .unwrap();

        let detector = PythonDetector::new();
        let result = detector.detect_python_setuptools(path);

        // Should fail since setup.py is required as primary manifest
        assert!(result.is_err());
    }

    #[test]
    fn test_case_sensitivity_in_content_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        fs::write(
            path.join("pyproject.toml"),
            r#"[BUILD-SYSTEM]
requires = ["setuptools"]

[TOOL.POETRY]
name = "uppercase-package"
version = "0.1.0"
"#,
        )
        .unwrap();

        let detector = PythonDetector::new();
        let detection = detector.detect_python_pyproject(path).unwrap();

        assert!(matches!(detection.framework, Framework::Python));
        // Should still detect despite case differences
        assert!(detection.confidence > 0.5);
    }
}
