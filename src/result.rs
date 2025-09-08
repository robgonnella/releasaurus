//! Error handling and result types for Releasaurus.
//!
//! This module provides a unified error handling approach using the `color-eyre` crate,
//! which offers enhanced error reporting with context, suggestions, and colored output.
//!
//! All functions in Releasaurus that can fail should return the `Result<T>` type defined
//! in this module, ensuring consistent error handling and reporting across the application.
//!
//! # Features
//!
//! - **Enhanced Error Display**: Automatic colorized error output with context
//! - **Error Suggestions**: Helpful suggestions for common error scenarios
//! - **Stack Traces**: Optional stack trace information for debugging
//! - **Error Context**: Ability to add context to errors as they propagate
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::result::Result;
//!
//! fn example_function() -> Result<String> {
//!     // Operations that might fail...
//!     Ok("success".to_string())
//! }
//! ```

use color_eyre::eyre::Result as EyreResult;

/// Standard result type used throughout Releasaurus.
///
/// This is a type alias for `color_eyre::eyre::Result<T>`, providing enhanced
/// error reporting capabilities including:
///
/// - Colorized error output in terminals
/// - Automatic error context and suggestions
/// - Optional stack trace information
/// - Chain-able error contexts using `.wrap_err()`
///
/// # Examples
///
/// ```rust,ignore
/// use crate::result::Result;
/// use color_eyre::eyre::{eyre, Context};
///
/// fn parse_config() -> Result<Config> {
///     let content = std::fs::read_to_string("config.toml")
///         .wrap_err("Failed to read configuration file")?;
///
///     let config = toml::from_str(&content)
///         .wrap_err("Failed to parse TOML configuration")?;
///
///     Ok(config)
/// }
/// ```
///
/// # Error Context
///
/// Use `.wrap_err()` to add context as errors propagate:
///
/// ```rust,ignore
/// fn process_release() -> Result<()> {
///     update_version_files()
///         .wrap_err("Failed to update version files during release")?;
///
///     generate_changelog()
///         .wrap_err("Failed to generate changelog")?;
///
///     Ok(())
/// }
/// ```
pub type Result<T> = EyreResult<T>;
