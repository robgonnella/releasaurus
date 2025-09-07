use log::*;
use std::{fs, path::Path};

use crate::{
    result::Result,
    updater::detection::types::{DetectionPattern, FrameworkDetection},
};

/// Helper for analyzing detection patterns and calculating confidence
pub struct DetectionHelper {}

impl DetectionHelper {
    /// Calculate confidence score based on evidence
    pub fn calculate_confidence(
        pattern: &DetectionPattern,
        evidence: &[String],
    ) -> f32 {
        let mut confidence = pattern.base_confidence;

        // Boost confidence for each piece of evidence
        // Subtract 1 for initial "base" evidence i.e. Cargo.toml, package.json
        // as this is what triggered the detection to begin with. Then add
        // 5% confidence for each additional piece of supporting evidence
        let evidence_boost = (evidence.len() as f32 - 1.0) * 0.05;
        confidence = (confidence + evidence_boost).min(1.0);

        confidence
    }

    /// Analyze a directory using a detection pattern
    pub fn analyze_with_pattern<
        F: FnOnce(Vec<String>) -> FrameworkDetection,
    >(
        path: &Path,
        pattern: DetectionPattern,
        create_detection: F,
    ) -> Result<FrameworkDetection> {
        // Look for primary manifest files
        let mut evidence = Vec::new();

        for manifest_file in &pattern.manifest_files {
            let manifest_path = path.join(manifest_file);
            if manifest_path.exists() {
                evidence.push(format!("found {}", manifest_file));

                // Read and analyze content
                match fs::read_to_string(&manifest_path) {
                    Ok(content) => {
                        // Check for content patterns
                        for pattern_text in &pattern.content_patterns {
                            if content.contains(pattern_text) {
                                evidence
                                    .push(format!("contains {}", pattern_text));
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to read {}: {}",
                            manifest_path.display(),
                            e
                        );
                    }
                }
            }
        }

        // If no manifest files found, this detection fails
        if evidence.is_empty() {
            return Err(color_eyre::eyre::eyre!("No manifest files found"));
        }

        // Look for supporting files
        for support_file in &pattern.support_files {
            let support_path = path.join(support_file);
            if support_path.exists() {
                evidence.push(format!("found {}", support_file));
            }
        }

        Ok(create_detection(evidence))
    }
}
