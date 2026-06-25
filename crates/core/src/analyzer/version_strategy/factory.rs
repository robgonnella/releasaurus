use crate::{
    analyzer::{
        config::AnalyzerConfig,
        version_strategy::{
            date::DateVersionStrategy,
            date_with_time::DateWithTimeVersionStrategy,
            date_with_time_micro::DateWithTimeMicroVersionStrategy,
            semantic::SemanticVersionStrategy,
            semantic_build::SemanticBuildVersionStrategy,
            traits::VersionStrategy,
        },
    },
    config::VersionType,
    result::Result,
};

/// Factory for creating version strategies based on configuration.
pub struct VersionStrategyFactory;

impl VersionStrategyFactory {
    /// Create a version strategy based on the provided analyzer configuration.
    pub fn create(config: &AnalyzerConfig) -> Result<Box<dyn VersionStrategy>> {
        match config.version_type {
            VersionType::Semantic => Ok(Box::new(SemanticVersionStrategy)),
            VersionType::SemanticWithBuild => {
                Ok(Box::new(SemanticBuildVersionStrategy))
            }
            VersionType::Date => Ok(Box::new(DateVersionStrategy)),
            VersionType::DateWithTime => {
                Ok(Box::new(DateWithTimeVersionStrategy))
            }
            VersionType::DateWithTimeMicro => {
                Ok(Box::new(DateWithTimeMicroVersionStrategy))
            }
        }
    }
}
