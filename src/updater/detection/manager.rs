use color_eyre::eyre::{Context, Result};
use log::*;
use std::path::{Path, PathBuf};

use crate::updater::{
    detection::{traits::FrameworkDetector, types::FrameworkDetection},
    framework::Framework,
};

pub struct DetectionManager {
    root_path: PathBuf,
    detectors: Vec<Box<dyn FrameworkDetector>>,
}

impl DetectionManager {
    pub fn new(
        root_path: PathBuf,
        detectors: Vec<Box<dyn FrameworkDetector>>,
    ) -> Self {
        Self {
            root_path,
            detectors,
        }
    }

    /// Detect the framework for a specific package path
    pub fn detect_framework(
        &self,
        package_path: &str,
    ) -> Result<FrameworkDetection> {
        let path = if package_path == "." {
            self.root_path.clone()
        } else {
            self.root_path.join(package_path)
        };

        debug!("Detecting framework for path: {}", path.display());

        let detection = self.analyze_directory(&path).with_context(|| {
            format!("Failed to analyze directory: {}", path.display())
        })?;

        Ok(detection)
    }

    /// Analyze a specific directory for framework indicators
    fn analyze_directory(&self, path: &Path) -> Result<FrameworkDetection> {
        if !path.is_dir() {
            return Ok(self.create_generic_detection(0.0, vec![]));
        }

        let mut best_detection = None;
        let mut best_confidence = 0.0;

        for detector in self.detectors.iter() {
            let detection_result = detector.detect(path);

            match detection_result {
                Ok(detection) => {
                    debug!(
                        "Framework {} detection confidence: {:.2}",
                        detection.framework.name(),
                        detection.confidence
                    );
                    if detection.confidence > best_confidence {
                        best_confidence = detection.confidence;
                        best_detection = Some(detection);
                    }
                }
                Err(e) => {
                    debug!("Failed to detect {}: {}", detector.name(), e);
                }
            }
        }

        Ok(best_detection.unwrap_or_else(|| {
            self.create_generic_detection(
                0.1,
                vec!["directory exists".to_string()],
            )
        }))
    }

    /// Create a generic detection for unknown frameworks
    fn create_generic_detection(
        &self,
        confidence: f32,
        evidence: Vec<String>,
    ) -> FrameworkDetection {
        FrameworkDetection {
            framework: Framework::Generic,
            confidence,
            evidence,
        }
    }
}
