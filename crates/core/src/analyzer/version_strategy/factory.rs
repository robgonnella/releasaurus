use crate::{
    analyzer::{
        config::AnalyzerConfig,
        version_strategy::{
            semantic::SemanticVersionStrategy,
            semantic_build::SemanticBuildVersionStrategy,
            semantic_date::DateVersionStrategy,
            semantic_date_with_time::DateWithTimeVersionStrategy,
            semantic_date_with_time_micro::DateWithTimeMicroVersionStrategy,
            strftime::StrftimeVersionStrategy, traits::VersionStrategy,
        },
    },
    config::versioning::VersionType,
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
            VersionType::SemanticDate => Ok(Box::new(DateVersionStrategy)),
            VersionType::SemanticDateWithTime => {
                Ok(Box::new(DateWithTimeVersionStrategy))
            }
            VersionType::SemanticDateWithTimeMicro => {
                Ok(Box::new(DateWithTimeMicroVersionStrategy))
            }
            VersionType::Strftime(ref fmt) => {
                Ok(Box::new(StrftimeVersionStrategy::new(fmt.clone())))
            }
        }
    }
}
