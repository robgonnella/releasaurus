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

pub struct NodeDetector {}

impl NodeDetector {
    pub fn new() -> Self {
        Self {}
    }
}

impl FrameworkDetector for NodeDetector {
    fn name(&self) -> &str {
        "node"
    }

    fn detect(&self, path: &Path) -> Result<FrameworkDetection> {
        let pattern = DetectionPattern {
            manifest_files: vec!["package.json"],
            support_files: vec![
                "node_modules",
                "package-lock.json",
                "yarn.lock",
                "pnpm-lock.yaml",
            ],
            content_patterns: vec!["\"name\":", "\"version\":", "\"scripts\":"],
            base_confidence: 0.8,
        };

        DetectionHelper::analyze_with_pattern(
            path,
            pattern.clone(),
            |support_evidence| FrameworkDetection {
                framework: Framework::Node,
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
    fn test_node_detection() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create package.json
        fs::write(
            path.join("package.json"),
            r#"{
  "name": "test-app",
  "version": "1.0.0",
  "scripts": {
    "test": "jest"
  },
  "dependencies": {
    "express": "^4.18.0"
  }
}"#,
        )
        .unwrap();

        // Create supporting files
        fs::write(path.join("package-lock.json"), "{}").unwrap();

        let detector = NodeDetector::new();
        let detection = detector.detect(path).unwrap();

        assert!(matches!(detection.framework, Framework::Node));

        assert!(detection.confidence > 0.7);

        assert!(
            detection
                .evidence
                .contains(&"found package.json".to_string())
        );
    }
}
