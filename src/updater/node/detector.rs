use std::path::Path;

use crate::updater::{
    detection::{
        helper::DetectionHelper,
        traits::FrameworkDetector,
        types::{DetectionPattern, FrameworkDetection},
    },
    framework::{Framework, Language},
    node::types::NodeMetadata,
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

    fn detect(
        &self,
        path: &Path,
    ) -> color_eyre::eyre::Result<FrameworkDetection> {
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
            |manifest_content, support_evidence| {
                let is_monorepo = manifest_content.contains("\"workspaces\":")
                    || path.join("lerna.json").exists()
                    || manifest_content.contains("\"nx\":");

                let monorepo_root = if is_monorepo {
                    Some(path.to_path_buf())
                } else {
                    None
                };

                // Detect package manager
                let package_manager = if path.join("pnpm-lock.yaml").exists() {
                    "pnpm".to_string()
                } else if path.join("yarn.lock").exists() {
                    "yarn".to_string()
                } else {
                    "npm".to_string()
                };

                let metadata = NodeMetadata {
                    is_monorepo,
                    monorepo_root,
                    package_manager,
                };

                FrameworkDetection {
                    framework: Framework::Node(Language {
                        name: self.name().into(),
                        manifest_path: path.join("package.json"),
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

        match detection.framework {
            Framework::Node(lang) => {
                assert!(!lang.metadata.is_monorepo);
                assert_eq!(lang.metadata.package_manager, "npm");
            }
            _ => panic!("Expected Node framework"),
        }

        assert!(detection.confidence > 0.7);
        assert!(
            detection
                .evidence
                .contains(&"found package.json".to_string())
        );
    }
}
