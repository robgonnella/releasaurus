use std::path::Path;

use crate::updater::{
    detection::{
        helper::DetectionHelper,
        traits::FrameworkDetector,
        types::{DetectionPattern, FrameworkDetection},
    },
    framework::Framework,
};
use color_eyre::eyre::Result;

pub struct RustDetector {}

impl RustDetector {
    pub fn new() -> Self {
        Self {}
    }
}

impl FrameworkDetector for RustDetector {
    fn name(&self) -> &str {
        "rust"
    }

    fn detect(&self, path: &Path) -> Result<FrameworkDetection> {
        let pattern = DetectionPattern {
            manifest_files: vec!["Cargo.toml"],
            support_files: vec!["Cargo.lock", "src/main.rs", "src/lib.rs"],
            content_patterns: vec![
                "[package]",
                "[workspace]",
                "[dependencies]",
            ],
            base_confidence: 0.9,
        };

        DetectionHelper::analyze_with_pattern(
            path,
            pattern.clone(),
            |support_evidence| FrameworkDetection {
                framework: Framework::Rust,
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
    fn test_rust_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create Cargo.toml
        fs::write(
            path.join("Cargo.toml"),
            r#"[package]
name = "test-crate"
version = "1.0.0"

[dependencies]
serde = "1.0"
"#,
        )
        .unwrap();

        // Create supporting files
        fs::create_dir_all(path.join("src")).unwrap();
        fs::write(path.join("src/lib.rs"), "// test").unwrap();

        let detector = RustDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Rust));

        assert!(detection.confidence > 0.8);

        assert!(detection.evidence.contains(&"found Cargo.toml".to_string()));
    }

    #[test]
    fn test_workspace_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create workspace Cargo.toml
        fs::write(
            path.join("Cargo.toml"),
            r#"[workspace]
members = [
    "crates/core",
    "crates/cli"
]

[workspace.dependencies]
serde = "1.0"
"#,
        )
        .unwrap();

        let detector = RustDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Rust));

        assert!(
            detection
                .evidence
                .contains(&"contains [workspace]".to_string())
        );
    }
}
