//! TOML configuration types and runtime overrides.
//!
//! The root TOML config is [`Config`]. Callers layer runtime
//! overrides from [`overrides`] on top of it, and
//! [`Resolver`][crate::resolver::Resolver] merges the two with forge
//! metadata to produce the types the pipeline actually runs on:
//! [`ResolvedConfig`][crate::resolver::ResolvedConfig]
//! and its [`ResolvedPackage`][crate::resolver::ResolvedPackage]s.

pub mod changelog;
pub mod defaults;
pub mod overrides;
pub mod package;
pub mod prerelease;
pub mod release_type;
pub mod repository;
mod toml;
pub mod versioning;

pub use toml::{
    Config, ConfigBuilder, ConfigBuilderError, DEFAULT_CONFIG_FILE,
};
