//! TOML configuration types and runtime-resolved variants.
//!
//! The root TOML config is [`Config`]. After applying CLI overrides
//! and forge metadata, [`resolved::ResolvedConfig`] is the type
//! used throughout the pipeline.

pub mod changelog;
pub mod global;
pub mod package;
pub mod prerelease;
pub mod release_type;
pub mod repository;
pub mod resolved;
mod toml;

pub use toml::{
    Config, ConfigBuilder, ConfigBuilderError, DEFAULT_CONFIG_FILE,
};
