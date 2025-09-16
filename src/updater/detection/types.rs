use crate::updater::framework::Framework;

/// Framework detection result
#[derive(Debug, Clone, PartialEq)]
/// Result of framework detection with confidence score and evidence.
pub struct FrameworkDetection {
    /// Detected framework
    pub framework: Framework,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Files that contributed to detection
    pub evidence: Vec<String>,
}

/// Detection patterns for different frameworks
#[derive(Clone)]
/// Pattern definition for detecting specific frameworks.
pub struct DetectionPattern<'a> {
    /// Primary manifest files that indicate this framework
    pub manifest_files: Vec<&'a str>,
    /// Secondary files that support the detection
    pub support_files: Vec<&'a str>,
    /// Content patterns to look for in manifest files
    pub content_patterns: Vec<&'a str>,
    /// Minimum confidence score for this pattern
    pub base_confidence: f32,
}
